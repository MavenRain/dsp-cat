//! Decimator DSP block.
//!
//! Passes every Nth sample, discarding the rest.

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op, WireTy};
use hdl_cat::kind::BitSeq;

use crate::error::Error;
use crate::hdl::common::{bits_for_value, const_unsigned, SAMPLE_WIRE, VALID_WIRE};
use crate::hdl::raw::RawDspBlock;

/// Build a decimator block with the given factor.
///
/// State: counter (`Bits(counter_width)`).
/// Latency: combinational (data passes through; valid is gated).
///
/// At each cycle:
/// - `data_out = data_in` (passthrough)
/// - If `valid_in` and `counter == 0`: `valid_out = true`
/// - Otherwise: `valid_out = false`
/// - Counter advances only on `valid_in`, wraps at `factor`.
///
/// # Errors
///
/// Returns [`Error::InvalidRateFactor`] if `factor` is zero.
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn build_decimator(factor: usize) -> Result<RawDspBlock, Error> {
    if factor == 0 {
        return Err(Error::InvalidRateFactor { factor });
    }

    let cw = bits_for_value(factor);

    // State wire: counter.
    let (builder, counter) = HdlGraphBuilder::new().with_wire(WireTy::Bits(cw));

    // Input wires.
    let (builder, data_in) = builder.with_wire(SAMPLE_WIRE);
    let (builder, valid_in) = builder.with_wire(VALID_WIRE);

    // Constants.
    let (builder, one) = const_unsigned(builder, 1, cw)?;
    let (builder, zero) = const_unsigned(builder, 0, cw)?;
    let (builder, factor_const) = const_unsigned(builder, u128::try_from(factor).unwrap_or(0), cw)?;

    // raw_next = counter + 1 (wrapping at counter_width bits).
    let (builder, raw_next) = builder.with_wire(WireTy::Bits(cw));
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Add),
        vec![counter, one],
        raw_next,
    )?;

    // at_factor = (raw_next == factor).
    let (builder, at_factor) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Eq),
        vec![raw_next, factor_const],
        at_factor,
    )?;

    // wrapped_next = Mux(at_factor, raw_next, zero).
    let (builder, wrapped_next) = builder.with_wire(WireTy::Bits(cw));
    let builder = builder.with_instruction(
        Op::Mux,
        vec![at_factor, raw_next, zero],
        wrapped_next,
    )?;

    // next_counter = Mux(valid_in, counter, wrapped_next).
    // Only advance when valid_in is true.
    let (builder, next_counter) = builder.with_wire(WireTy::Bits(cw));
    let builder = builder.with_instruction(
        Op::Mux,
        vec![valid_in, counter, wrapped_next],
        next_counter,
    )?;

    // counter_is_zero = (counter == 0).
    let (builder, counter_is_zero) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Eq),
        vec![counter, zero],
        counter_is_zero,
    )?;

    // decimated_valid = valid_in AND counter_is_zero.
    let (builder, decimated_valid) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::And),
        vec![valid_in, counter_is_zero],
        decimated_valid,
    )?;

    let graph = builder.build();

    let initial_counter: BitSeq = (0..cw).map(|_| false).collect();

    let input_wires = vec![counter, data_in, valid_in];
    let output_wires = vec![next_counter, data_in, decimated_valid];

    Ok(RawDspBlock::new(graph, input_wires, output_wires, initial_counter, 1))
}

#[cfg(test)]
mod tests {
    use super::{bits_for_value, build_decimator};

    #[test]
    fn bits_for_value_cases() {
        assert_eq!(bits_for_value(0), 1);
        assert_eq!(bits_for_value(1), 1);
        assert_eq!(bits_for_value(2), 2);
        assert_eq!(bits_for_value(3), 2);
        assert_eq!(bits_for_value(4), 3);
        assert_eq!(bits_for_value(255), 8);
        assert_eq!(bits_for_value(256), 9);
    }

    #[test]
    fn decimator_factor_2_has_one_state_wire() -> Result<(), crate::error::Error> {
        let block = build_decimator(2)?;
        assert_eq!(block.state_wire_count(), 1);
        Ok(())
    }

    #[test]
    fn decimator_factor_zero_errors() {
        assert!(build_decimator(0).is_err());
    }

    #[test]
    fn decimator_factor_1_is_passthrough() -> Result<(), crate::error::Error> {
        let block = build_decimator(1)?;
        assert_eq!(block.state_wire_count(), 1);
        Ok(())
    }
}
