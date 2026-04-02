//! CIC filter internal FSM graph.
//!
//! Models the per-sample processing cycle: cascaded integrators,
//! decimation check, then cascaded combs.

use comp_cat_rs::collapse::free_category::{Edge, FreeCategoryError, Graph, Path, Vertex};

/// Number of vertices in the CIC FSM graph.
pub const CIC_VERTICES: usize = 7;

/// Number of edges in the CIC FSM graph.
pub const CIC_EDGES: usize = 8;

/// Edge table: `(source, target)` for each edge.
///
/// ```text
/// 0:Input -> 1:Integrate -> 2:CheckInt
///             ^                  |
///             |__(more stages)___|
///                                |
///            3:Decimate <--------|
///                |
///            4:Comb -> 5:CheckComb
///             ^             |
///             |__(more)_____|
///                           |
///                       6:Output
/// ```
const EDGE_TABLE: [(usize, usize); CIC_EDGES] = [
    (0, 1), // Input -> Integrate
    (1, 2), // Integrate -> CheckIntStages
    (2, 1), // CheckIntStages -> Integrate (loop)
    (2, 3), // CheckIntStages -> Decimate (done)
    (3, 4), // Decimate -> Comb
    (4, 5), // Comb -> CheckCombStages
    (5, 4), // CheckCombStages -> Comb (loop)
    (5, 6), // CheckCombStages -> Output (done)
];

/// The CIC filter internal FSM graph.
#[derive(Debug, Clone, Copy)]
pub struct CicGraph;

impl Graph for CicGraph {
    fn vertex_count(&self) -> usize {
        CIC_VERTICES
    }

    fn edge_count(&self) -> usize {
        CIC_EDGES
    }

    fn source(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        EDGE_TABLE
            .get(edge.index())
            .map(|(s, _)| Vertex::new(*s))
            .ok_or(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: CIC_EDGES,
            })
    }

    fn target(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        EDGE_TABLE
            .get(edge.index())
            .map(|(_, t)| Vertex::new(*t))
            .ok_or(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: CIC_EDGES,
            })
    }
}

/// The single-sample path without loops:
/// Input -> Integrate -> `CheckInt` -> Decimate -> Comb -> `CheckComb` -> Output.
///
/// # Errors
///
/// Returns a [`FreeCategoryError`] if path construction fails.
pub fn single_pass_path() -> Result<Path, FreeCategoryError> {
    let g = CicGraph;
    [0, 1, 3, 4, 5, 7]
        .iter()
        .map(|&k| Path::singleton(&g, Edge::new(k)))
        .try_fold(Path::identity(Vertex::new(0)), |acc, edge_path| {
            acc.compose(edge_path?)
        })
}

/// The integrator loop sub-path: `CheckInt` -> Integrate -> `CheckInt`.
///
/// # Errors
///
/// Returns a [`FreeCategoryError`] if path construction fails.
pub fn integrator_loop_path() -> Result<Path, FreeCategoryError> {
    let g = CicGraph;
    [2, 1]
        .iter()
        .map(|&k| Path::singleton(&g, Edge::new(k)))
        .try_fold(Path::identity(Vertex::new(2)), |acc, edge_path| {
            acc.compose(edge_path?)
        })
}

/// The comb loop sub-path: `CheckComb` -> Comb -> `CheckComb`.
///
/// # Errors
///
/// Returns a [`FreeCategoryError`] if path construction fails.
pub fn comb_loop_path() -> Result<Path, FreeCategoryError> {
    let g = CicGraph;
    [6, 5]
        .iter()
        .map(|&k| Path::singleton(&g, Edge::new(k)))
        .try_fold(Path::identity(Vertex::new(5)), |acc, edge_path| {
            acc.compose(edge_path?)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_has_correct_dimensions() {
        let g = CicGraph;
        assert_eq!(g.vertex_count(), CIC_VERTICES);
        assert_eq!(g.edge_count(), CIC_EDGES);
    }

    #[test]
    fn all_edges_have_valid_endpoints() -> Result<(), FreeCategoryError> {
        let g = CicGraph;
        (0..CIC_EDGES).try_for_each(|k| {
            let s = g.source(Edge::new(k))?;
            let t = g.target(Edge::new(k))?;
            assert!(s.index() < CIC_VERTICES);
            assert!(t.index() < CIC_VERTICES);
            Ok(())
        })
    }

    #[test]
    fn out_of_bounds_edge_is_error() {
        let g = CicGraph;
        assert!(g.source(Edge::new(CIC_EDGES)).is_err());
    }

    #[test]
    fn single_pass_goes_input_to_output() -> Result<(), FreeCategoryError> {
        let path = single_pass_path()?;
        assert_eq!(path.source().index(), 0);
        assert_eq!(path.target().index(), 6);
        Ok(())
    }

    #[test]
    fn integrator_loop_is_round_trip() -> Result<(), FreeCategoryError> {
        let path = integrator_loop_path()?;
        assert_eq!(path.source().index(), 2);
        assert_eq!(path.target().index(), 2);
        Ok(())
    }

    #[test]
    fn comb_loop_is_round_trip() -> Result<(), FreeCategoryError> {
        let path = comb_loop_path()?;
        assert_eq!(path.source().index(), 5);
        assert_eq!(path.target().index(), 5);
        Ok(())
    }
}
