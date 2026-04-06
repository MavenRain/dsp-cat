//! Accumulator (running sum) DSP block.
//!
//! Single integrator stage: `y[n] = y[n-1] + x[n]`.
//!
//! Wrapping addition (standard DSP hardware semantics).

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op};

use crate::error::Error;
use crate::hdl::common::{zero_sample_init, zero_valid_init, SAMPLE_WIRE, VALID_WIRE};
use crate::hdl::raw::RawDspBlock;

/// Build a running-sum accumulator.
///
/// State: accumulated value (`Signed(32)`) + previous valid (`Bit`).
/// Latency: 1 clock cycle (registered valid output).
///
/// At each cycle:
/// - `sum = accum + data_in` (wrapping)
/// - `next_accum = if valid_in { sum } else { accum }`
/// - `data_out = sum`
/// - `valid_out = prev_valid`
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn build_accumulator() -> Result<RawDspBlock, Error> {
    // State wires.
    let (builder, accum) = HdlGraphBuilder::new().with_wire(SAMPLE_WIRE);
    let (builder, prev_valid) = builder.with_wire(VALID_WIRE);

    // Input wires.
    let (builder, data_in) = builder.with_wire(SAMPLE_WIRE);
    let (builder, valid_in) = builder.with_wire(VALID_WIRE);

    // sum = accum + data_in (wrapping).
    let (builder, sum) = builder.with_wire(SAMPLE_WIRE);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Add),
        vec![accum, data_in],
        sum,
    )?;

    // next_accum = Mux(valid_in, accum, sum).
    let (builder, next_accum) = builder.with_wire(SAMPLE_WIRE);
    let builder = builder.with_instruction(
        Op::Mux,
        vec![valid_in, accum, sum],
        next_accum,
    )?;

    let graph = builder.build();

    // State wires first, then data I/O.
    let input_wires = vec![accum, prev_valid, data_in, valid_in];
    let output_wires = vec![next_accum, valid_in, sum, prev_valid];
    let initial_state = zero_sample_init().concat(zero_valid_init());

    Ok(RawDspBlock::new(graph, input_wires, output_wires, initial_state, 2))
}

#[cfg(test)]
mod tests {
    use super::build_accumulator;

    #[test]
    fn accumulator_has_two_state_wires() -> Result<(), crate::error::Error> {
        let block = build_accumulator()?;
        assert_eq!(block.state_wire_count(), 2);
        assert_eq!(block.input_wires().len(), 4);
        assert_eq!(block.output_wires().len(), 4);
        Ok(())
    }

    #[test]
    fn accumulator_initial_state_is_33_bits() -> Result<(), crate::error::Error> {
        let block = build_accumulator()?;
        // 32 bits for sample + 1 bit for valid.
        assert_eq!(block.initial_state().len(), 33);
        Ok(())
    }
}
