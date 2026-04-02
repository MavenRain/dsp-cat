//! Top-level pipeline topology graph.
//!
//! A linear chain of DSP blocks: N+1 vertices (inter-block boundaries)
//! connected by N edges (DSP blocks).  Edge k connects vertex k to
//! vertex k+1.

use comp_cat_rs::collapse::free_category::{Edge, FreeCategoryError, Graph, Path, Vertex};

/// A linear pipeline graph with `block_count` edges.
///
/// Vertex k represents the signal boundary after k processing blocks.
/// Edge k represents the k-th DSP block.
///
/// # Examples
///
/// ```
/// use dsp_cat::graph::pipeline_graph::PipelineGraph;
/// use comp_cat_rs::collapse::free_category::Graph;
///
/// let g = PipelineGraph::new(3);
/// assert_eq!(g.vertex_count(), 4);
/// assert_eq!(g.edge_count(), 3);
/// ```
#[derive(Debug, Clone)]
pub struct PipelineGraph {
    block_count: usize,
}

impl PipelineGraph {
    /// Create a pipeline graph with the given number of blocks.
    #[must_use]
    pub fn new(block_count: usize) -> Self {
        Self { block_count }
    }

    /// The number of blocks (edges) in this pipeline.
    #[must_use]
    pub fn block_count(&self) -> usize {
        self.block_count
    }
}

impl Graph for PipelineGraph {
    fn vertex_count(&self) -> usize {
        self.block_count + 1
    }

    fn edge_count(&self) -> usize {
        self.block_count
    }

    fn source(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        if edge.index() < self.block_count {
            Ok(Vertex::new(edge.index()))
        } else {
            Err(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: self.block_count,
            })
        }
    }

    fn target(&self, edge: Edge) -> Result<Vertex, FreeCategoryError> {
        if edge.index() < self.block_count {
            Ok(Vertex::new(edge.index() + 1))
        } else {
            Err(FreeCategoryError::EdgeOutOfBounds {
                edge,
                count: self.block_count,
            })
        }
    }
}

/// Build the full pipeline path through all blocks.
///
/// # Errors
///
/// Returns a [`FreeCategoryError`] if path construction fails.
///
/// # Examples
///
/// ```
/// use dsp_cat::graph::pipeline_graph::{PipelineGraph, full_pipeline_path};
///
/// let g = PipelineGraph::new(5);
/// let path = full_pipeline_path(&g).ok();
/// assert_eq!(path.map(|p| p.len()), Some(5));
/// ```
pub fn full_pipeline_path(graph: &PipelineGraph) -> Result<Path, FreeCategoryError> {
    (0..graph.block_count())
        .map(|k| Path::singleton(graph, Edge::new(k)))
        .try_fold(Path::identity(Vertex::new(0)), |acc, edge_path| {
            acc.compose(edge_path?)
        })
}

/// Build a sub-pipeline path from block `start` to block `end` (exclusive).
///
/// # Errors
///
/// Returns a [`FreeCategoryError`] if any edge is out of bounds or
/// path composition fails.
pub fn sub_pipeline_path(
    graph: &PipelineGraph,
    start: usize,
    end: usize,
) -> Result<Path, FreeCategoryError> {
    (start..end)
        .map(|k| Path::singleton(graph, Edge::new(k)))
        .try_fold(Path::identity(Vertex::new(start)), |acc, edge_path| {
            acc.compose(edge_path?)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_blocks_has_one_vertex() {
        let g = PipelineGraph::new(0);
        assert_eq!(g.vertex_count(), 1);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn edge_connects_adjacent_vertices() -> Result<(), FreeCategoryError> {
        let g = PipelineGraph::new(5);
        (0..5).try_for_each(|k| {
            let s = g.source(Edge::new(k))?;
            let t = g.target(Edge::new(k))?;
            assert_eq!(s.index(), k);
            assert_eq!(t.index(), k + 1);
            Ok(())
        })
    }

    #[test]
    fn out_of_bounds_edge_is_error() {
        let g = PipelineGraph::new(3);
        assert!(g.source(Edge::new(3)).is_err());
        assert!(g.target(Edge::new(5)).is_err());
    }

    #[test]
    fn full_path_spans_all_blocks() -> Result<(), FreeCategoryError> {
        let g = PipelineGraph::new(4);
        let path = full_pipeline_path(&g)?;
        assert_eq!(path.source().index(), 0);
        assert_eq!(path.target().index(), 4);
        assert_eq!(path.len(), 4);
        Ok(())
    }

    #[test]
    fn sub_path_spans_range() -> Result<(), FreeCategoryError> {
        let g = PipelineGraph::new(6);
        let path = sub_pipeline_path(&g, 2, 5)?;
        assert_eq!(path.source().index(), 2);
        assert_eq!(path.target().index(), 5);
        assert_eq!(path.len(), 3);
        Ok(())
    }

    #[test]
    fn empty_pipeline_full_path_is_identity() -> Result<(), FreeCategoryError> {
        let g = PipelineGraph::new(0);
        let path = full_pipeline_path(&g)?;
        assert!(path.is_identity());
        Ok(())
    }

    #[test]
    fn sub_paths_compose_to_full() -> Result<(), FreeCategoryError> {
        let g = PipelineGraph::new(6);
        let left = sub_pipeline_path(&g, 0, 3)?;
        let right = sub_pipeline_path(&g, 3, 6)?;
        let composed = left.compose(right)?;
        let full = full_pipeline_path(&g)?;
        assert_eq!(composed.source(), full.source());
        assert_eq!(composed.target(), full.target());
        assert_eq!(composed.len(), full.len());
        Ok(())
    }
}
