//! FIR filter internal FSM graph.
//!
//! Models the per-sample processing cycle of a FIR filter:
//! idle, shift sample into tap chain, multiply-accumulate across
//! all taps, then output.

use comp_cat_rs::collapse::free_category::{Edge, FreeCategoryError, Graph, Path, Vertex};

/// Number of vertices in the FIR FSM graph.
pub const FIR_VERTICES: usize = 6;

/// Number of edges in the FIR FSM graph.
pub const FIR_EDGES: usize = 7;

/// Edge table: `(source, target)` for each edge.
///
/// ```text
/// 0:Idle -> 1:Load -> 2:Shift -> 3:Mac -> 4:Check
///                      ^                     |
///                      |_____(more taps)_____|
///                                            |
///                                        5:Output -> 0:Idle
/// ```
const EDGE_TABLE: [(usize, usize); FIR_EDGES] = [
    (0, 1), // Idle -> LoadCoeff
    (1, 2), // LoadCoeff -> Shift
    (2, 3), // Shift -> Mac
    (3, 4), // Mac -> CheckTaps
    (4, 2), // CheckTaps -> Shift (loop: more taps)
    (4, 5), // CheckTaps -> Output (done)
    (5, 0), // Output -> Idle
];

/// The FIR filter internal FSM graph.
#[derive(Debug, Clone, Copy)]
pub struct FirGraph;

impl Graph for FirGraph {
    fn vertex_count(&self) -> usize {
        FIR_VERTICES
    }

    fn edge_count(&self) -> usize {
        FIR_EDGES
    }

    fn source(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        EDGE_TABLE
            .get(edge.index())
            .map(|(s, _)| Vertex::new(*s))
            .ok_or(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: FIR_EDGES,
            })
    }

    fn target(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        EDGE_TABLE
            .get(edge.index())
            .map(|(_, t)| Vertex::new(*t))
            .ok_or(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: FIR_EDGES,
            })
    }
}

/// The single-sample processing path (without the MAC loop):
/// Idle -> Load -> Shift -> Mac -> Check -> Output -> Idle.
///
/// # Errors
///
/// Returns a [`FreeCategoryError`] if path construction fails.
pub fn single_pass_path() -> Result<Path, FreeCategoryError> {
    let g = FirGraph;
    [0, 1, 2, 3, 5, 6]
        .iter()
        .map(|&k| Path::singleton(&g, Edge::new(k)))
        .try_fold(Path::identity(Vertex::new(0)), |acc, edge_path| {
            acc.compose(edge_path?)
        })
}

/// The MAC loop sub-path: Check -> Shift -> Mac -> Check.
///
/// # Errors
///
/// Returns a [`FreeCategoryError`] if path construction fails.
pub fn mac_loop_path() -> Result<Path, FreeCategoryError> {
    let g = FirGraph;
    [4, 2, 3]
        .iter()
        .map(|&k| Path::singleton(&g, Edge::new(k)))
        .try_fold(Path::identity(Vertex::new(4)), |acc, edge_path| {
            acc.compose(edge_path?)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_has_correct_dimensions() {
        let g = FirGraph;
        assert_eq!(g.vertex_count(), FIR_VERTICES);
        assert_eq!(g.edge_count(), FIR_EDGES);
    }

    #[test]
    fn all_edges_have_valid_endpoints() -> Result<(), FreeCategoryError> {
        let g = FirGraph;
        (0..FIR_EDGES).try_for_each(|k| {
            let s = g.source(Edge::new(k))?;
            let t = g.target(Edge::new(k))?;
            assert!(s.index() < FIR_VERTICES);
            assert!(t.index() < FIR_VERTICES);
            Ok(())
        })
    }

    #[test]
    fn out_of_bounds_edge_is_error() {
        let g = FirGraph;
        assert!(g.source(Edge::new(FIR_EDGES)).is_err());
    }

    #[test]
    fn single_pass_is_round_trip() -> Result<(), FreeCategoryError> {
        let path = single_pass_path()?;
        assert_eq!(path.source().index(), 0);
        assert_eq!(path.target().index(), 0);
        Ok(())
    }

    #[test]
    fn mac_loop_starts_and_ends_at_check() -> Result<(), FreeCategoryError> {
        let path = mac_loop_path()?;
        assert_eq!(path.source().index(), 4);
        assert_eq!(path.target().index(), 4);
        Ok(())
    }
}
