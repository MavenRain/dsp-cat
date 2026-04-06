//! Gain (scale) DSP block.
//!
//! Multiplies each input sample by a fixed coefficient with
//! right-shift for fixed-point scaling.  1-cycle registered output.

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op, WireTy};

use crate::error::Error;
use crate::hdl::common::{
    arith_shr, const_signed_32, sign_extend, truncate,
    zero_sample_init, zero_valid_init, SAMPLE_WIRE, VALID_WIRE,
};
use crate::hdl::raw::RawDspBlock;

/// Build a gain block with fixed coefficient and right-shift.
///
/// State: registered output (`Signed(32)`) + registered valid (`Bit`).
/// Latency: 1 clock cycle.
///
/// At each cycle:
/// - `product = sign_extend(data_in) * sign_extend(coefficient)` (64-bit)
/// - `shifted = product >>> shift` (arithmetic right-shift)
/// - `next_out = truncate_32(shifted)`
/// - `next_valid = valid_in`
/// - `data_out = out_reg` (previous cycle's registered output)
/// - `valid_out = valid_reg` (previous cycle's registered valid)
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn build_gain(coefficient: i32, shift: u32) -> Result<RawDspBlock, Error> {
    // State wires (registered output).
    let (builder, out_reg) = HdlGraphBuilder::new().with_wire(SAMPLE_WIRE);
    let (builder, valid_reg) = builder.with_wire(VALID_WIRE);

    // Input wires.
    let (builder, data_in) = builder.with_wire(SAMPLE_WIRE);
    let (builder, valid_in) = builder.with_wire(VALID_WIRE);

    // Coefficient constant.
    let (builder, coeff) = const_signed_32(builder, coefficient)?;

    // Sign-extend both operands to 64-bit for widening multiply.
    let (builder, data_ext) = sign_extend(builder, data_in, 32, 64)?;
    let (builder, coeff_ext) = sign_extend(builder, coeff, 32, 64)?;

    // 64-bit multiply (wrapping; no overflow since inputs are 32-bit).
    let (builder, product) = builder.with_wire(WireTy::Signed(64));
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Mul),
        vec![data_ext, coeff_ext],
        product,
    )?;

    // Arithmetic right-shift by the configured amount.
    let (builder, shifted) = arith_shr(builder, product, shift, 64)?;

    // Truncate back to 32 bits.
    let (builder, next_out) = truncate(builder, shifted, 32)?;

    let graph = builder.build();

    let input_wires = vec![out_reg, valid_reg, data_in, valid_in];
    let output_wires = vec![next_out, valid_in, out_reg, valid_reg];
    let initial_state = zero_sample_init().concat(zero_valid_init());

    Ok(RawDspBlock::new(graph, input_wires, output_wires, initial_state, 2))
}

#[cfg(test)]
mod tests {
    use super::build_gain;

    #[test]
    fn gain_has_two_state_wires() -> Result<(), crate::error::Error> {
        let block = build_gain(3, 0)?;
        assert_eq!(block.state_wire_count(), 2);
        Ok(())
    }

    #[test]
    fn gain_initial_state_is_33_bits() -> Result<(), crate::error::Error> {
        let block = build_gain(5, 8)?;
        assert_eq!(block.initial_state().len(), 33);
        Ok(())
    }
}
