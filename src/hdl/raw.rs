//! Type-erased DSP block representation and raw composition.
//!
//! [`RawDspBlock`] stores the IR graph, wire layout, and initial
//! state for a single DSP block (or a composed pipeline) without
//! encoding the state type at the Rust type level.  This enables
//! runtime pipeline construction from [`DspBlockDescriptor`]s.
//!
//! [`compose_raw`] replicates [`hdl_cat::sync::compose_sync`]'s
//! graph-merge logic at the raw level, enabling N-ary folded
//! composition without deeply nested `CircuitTensor` types.

use hdl_cat::ir::{HdlGraph, HdlGraphBuilder, WireId};
use hdl_cat::kind::BitSeq;

use crate::error::Error;
use crate::hdl::common::{SAMPLE_WIRE, VALID_WIRE};

/// A type-erased DSP block backed by an [`HdlGraph`].
///
/// The first `state_wire_count` entries of both [`input_wires`]
/// and [`output_wires`] are state-carrying wires.  The remaining
/// entries are the data I/O (one `Signed(32)` sample + one `Bit`
/// valid flag each).
#[derive(Clone, Debug)]
#[must_use]
pub struct RawDspBlock {
    graph: HdlGraph,
    input_wires: Vec<WireId>,
    output_wires: Vec<WireId>,
    initial_state: BitSeq,
    state_wire_count: usize,
}

impl RawDspBlock {
    /// Construct a new raw block from its constituent parts.
    pub fn new(
        graph: HdlGraph,
        input_wires: Vec<WireId>,
        output_wires: Vec<WireId>,
        initial_state: BitSeq,
        state_wire_count: usize,
    ) -> Self {
        Self {
            graph,
            input_wires,
            output_wires,
            initial_state,
            state_wire_count,
        }
    }

    /// The combinational IR graph.
    pub fn graph(&self) -> &HdlGraph {
        &self.graph
    }

    /// Input wires: `[state..., data_in, valid_in]`.
    #[must_use]
    pub fn input_wires(&self) -> &[WireId] {
        &self.input_wires
    }

    /// Output wires: `[next_state..., data_out, valid_out]`.
    #[must_use]
    pub fn output_wires(&self) -> &[WireId] {
        &self.output_wires
    }

    /// The initial-state bit pattern (LSB-first).
    pub fn initial_state(&self) -> &BitSeq {
        &self.initial_state
    }

    /// Number of state wires.
    #[must_use]
    pub fn state_wire_count(&self) -> usize {
        self.state_wire_count
    }

    /// Consume and return owned parts.
    pub fn into_parts(
        self,
    ) -> (HdlGraph, Vec<WireId>, Vec<WireId>, BitSeq, usize) {
        (
            self.graph,
            self.input_wires,
            self.output_wires,
            self.initial_state,
            self.state_wire_count,
        )
    }
}

/// An identity (passthrough) DSP block.
///
/// Output equals input; no state.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn identity_raw() -> Result<RawDspBlock, Error> {
    let (builder, data) = HdlGraphBuilder::new().with_wire(SAMPLE_WIRE);
    let (builder, valid) = builder.with_wire(VALID_WIRE);
    let graph = builder.build();
    Ok(RawDspBlock::new(
        graph,
        vec![data, valid],
        vec![data, valid],
        BitSeq::new(),
        0,
    ))
}

/// Sequentially compose two [`RawDspBlock`]s.
///
/// The first block's data output is wired into the second block's
/// data input.  State becomes the concatenation of both blocks'
/// state.  This mirrors [`hdl_cat::sync::compose_sync`].
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph merging fails.
#[allow(clippy::similar_names)]
pub fn compose_raw(f: RawDspBlock, g: RawDspBlock) -> Result<RawDspBlock, Error> {
    let (f_graph, f_inputs, f_outputs, f_init, f_sc) = f.into_parts();
    let (g_graph, g_inputs, g_outputs, g_init, g_sc) = g.into_parts();
    let f_wire_count = f_graph.wires().len();

    let shift = |w: WireId| WireId::new(w.index() + f_wire_count);

    let (state_f_input, data_f_input) = f_inputs.split_at(f_sc);
    let (state_f_output, data_f_output) = f_outputs.split_at(f_sc);
    let (state_g_input, data_g_input) = g_inputs.split_at(g_sc);
    let (state_g_output, data_g_output) = g_outputs.split_at(g_sc);

    // Build substitution map: g's data inputs -> f's data outputs.
    let substitution: Vec<(WireId, WireId)> = data_g_input
        .iter()
        .zip(data_f_output.iter())
        .map(|(g_in, f_out)| (shift(*g_in), *f_out))
        .collect();

    let remap_g = |w: WireId| -> WireId {
        let shifted = WireId::new(w.index() + f_wire_count);
        substitution
            .iter()
            .find_map(|(from, to)| (*from == shifted).then_some(*to))
            .unwrap_or(shifted)
    };

    let merged = merge_graphs(&f_graph, &g_graph, &remap_g)?;

    let combined_inputs: Vec<WireId> = state_f_input
        .iter()
        .copied()
        .chain(state_g_input.iter().copied().map(shift))
        .chain(data_f_input.iter().copied())
        .collect();

    let combined_outputs: Vec<WireId> = state_f_output
        .iter()
        .copied()
        .chain(state_g_output.iter().copied().map(shift))
        .chain(data_g_output.iter().copied().map(&remap_g))
        .collect();

    let combined_state = f_init.concat(g_init);
    let combined_state_count = f_sc + g_sc;

    Ok(RawDspBlock::new(
        merged,
        combined_inputs,
        combined_outputs,
        combined_state,
        combined_state_count,
    ))
}

fn merge_graphs(
    f_graph: &HdlGraph,
    g_graph: &HdlGraph,
    remap_g: &dyn Fn(WireId) -> WireId,
) -> Result<HdlGraph, Error> {
    // Copy all wires from both graphs.
    let builder = f_graph
        .wires()
        .iter()
        .cloned()
        .fold(HdlGraphBuilder::new(), |b, ty| b.with_wire(ty).0);
    let builder = g_graph
        .wires()
        .iter()
        .cloned()
        .fold(builder, |b, ty| b.with_wire(ty).0);

    // Copy f's instructions unchanged.
    let builder = f_graph
        .instructions()
        .iter()
        .try_fold(builder, |b, instr| {
            b.with_instruction(
                instr.op().clone(),
                instr.inputs().to_vec(),
                instr.output(),
            )
            .map_err(Error::from)
        })?;

    // Copy g's instructions with remapped wire IDs.
    let builder = g_graph
        .instructions()
        .iter()
        .try_fold(builder, |b, instr| {
            let new_inputs: Vec<WireId> = instr
                .inputs()
                .iter()
                .copied()
                .map(remap_g)
                .collect();
            let new_output = remap_g(instr.output());
            b.with_instruction(instr.op().clone(), new_inputs, new_output)
                .map_err(Error::from)
        })?;

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::{compose_raw, identity_raw, RawDspBlock};
    use crate::hdl::common::{zero_sample_init, SAMPLE_WIRE, VALID_WIRE};
    use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op};

    #[test]
    fn identity_has_no_state() -> Result<(), crate::error::Error> {
        let block = identity_raw()?;
        assert_eq!(block.state_wire_count(), 0);
        assert_eq!(block.initial_state().len(), 0);
        assert_eq!(block.input_wires().len(), 2);
        assert_eq!(block.output_wires().len(), 2);
        Ok(())
    }

    #[test]
    fn compose_two_identities_has_no_state() -> Result<(), crate::error::Error> {
        let a = identity_raw()?;
        let b = identity_raw()?;
        let c = compose_raw(a, b)?;
        assert_eq!(c.state_wire_count(), 0);
        assert_eq!(c.initial_state().len(), 0);
        Ok(())
    }

    fn simple_stateful_block() -> Result<RawDspBlock, crate::error::Error> {
        // A trivial block: state = Signed(32), pass data through,
        // next_state = state + data_in (wrapping).
        let (builder, state) = HdlGraphBuilder::new().with_wire(SAMPLE_WIRE);
        let (builder, data_in) = builder.with_wire(SAMPLE_WIRE);
        let (builder, valid_in) = builder.with_wire(VALID_WIRE);
        let (builder, sum) = builder.with_wire(SAMPLE_WIRE);
        let builder = builder.with_instruction(
            Op::Bin(BinOp::Add),
            vec![state, data_in],
            sum,
        )?;
        let graph = builder.build();
        Ok(RawDspBlock::new(
            graph,
            vec![state, data_in, valid_in],
            vec![sum, data_in, valid_in],
            zero_sample_init(),
            1,
        ))
    }

    #[test]
    fn compose_stateful_blocks_merges_state() -> Result<(), crate::error::Error> {
        let a = simple_stateful_block()?;
        let b = simple_stateful_block()?;
        let c = compose_raw(a, b)?;
        assert_eq!(c.state_wire_count(), 2);
        assert_eq!(c.initial_state().len(), 64);
        Ok(())
    }
}
