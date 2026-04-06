//! Interpolator DSP block.
//!
//! Inserts `factor - 1` zero-valued samples between each input
//! sample (zero-stuffing upsampler).

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op, WireTy};
use hdl_cat::kind::BitSeq;

use crate::error::Error;
use crate::hdl::common::{
    bits_for_value, const_bit, const_unsigned, i32_to_bit_seq, mux,
    SAMPLE_WIRE, VALID_WIRE,
};
use crate::hdl::raw::RawDspBlock;

/// Build an interpolator with the given factor.
///
/// State: held sample (`Signed(32)`), counter (`Bits(cw)`), active flag (`Bit`).
/// Omits the `ready` back-pressure signal (the pipeline does not use it).
///
/// When a valid input arrives, the block outputs the original sample
/// followed by `factor - 1` zeros on successive cycles.
///
/// # Errors
///
/// Returns [`Error::InvalidRateFactor`] if `factor` is zero.
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn build_interpolator(factor: usize) -> Result<RawDspBlock, Error> {
    if factor == 0 {
        return Err(Error::InvalidRateFactor { factor });
    }

    let cw = bits_for_value(factor);

    // ---- state wires ----
    let (builder, held) = HdlGraphBuilder::new().with_wire(SAMPLE_WIRE);
    let (builder, counter) = builder.with_wire(WireTy::Bits(cw));
    let (builder, active) = builder.with_wire(VALID_WIRE);

    // ---- input wires ----
    let (builder, data_in) = builder.with_wire(SAMPLE_WIRE);
    let (builder, valid_in) = builder.with_wire(VALID_WIRE);

    // ---- constants ----
    let (builder, zero_sample) = builder.with_wire(SAMPLE_WIRE);
    let builder = builder.with_instruction(
        Op::Const {
            bits: i32_to_bit_seq(0),
            ty: SAMPLE_WIRE,
        },
        vec![],
        zero_sample,
    )?;
    let (builder, zero_counter) = const_unsigned(builder, 0, cw)?;
    let (builder, one_counter) = const_unsigned(builder, 1, cw)?;
    let (builder, factor_const) = const_unsigned(
        builder,
        u128::try_from(factor).unwrap_or(0),
        cw,
    )?;
    let (builder, _const_true) = const_bit(builder, true)?;
    let (builder, const_false) = const_bit(builder, false)?;

    // Compile-time decision baked into the graph.
    let case2_counter_val = u128::from(factor > 1);
    let case2_active_val = factor > 1;
    let (builder, case2_counter) = const_unsigned(builder, case2_counter_val, cw)?;
    let (builder, case2_active) = const_bit(builder, case2_active_val)?;

    // ---- case 1 (active): output held or zero, advance counter ----

    // counter_is_zero = Eq(counter, zero_counter).
    let (builder, counter_is_zero) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Eq),
        vec![counter, zero_counter],
        counter_is_zero,
    )?;

    // data_case1 = Mux(counter_is_zero, zero_sample, held).
    let (builder, data_case1) = mux(builder, counter_is_zero, zero_sample, held, SAMPLE_WIRE)?;

    // counter_plus_one = counter + 1.
    let (builder, counter_plus_one) = builder.with_wire(WireTy::Bits(cw));
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Add),
        vec![counter, one_counter],
        counter_plus_one,
    )?;

    // at_factor = Eq(counter_plus_one, factor_const).
    let (builder, at_factor) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Eq),
        vec![counter_plus_one, factor_const],
        at_factor,
    )?;

    // next_counter_case1 = Mux(at_factor, counter_plus_one, zero_counter).
    let (builder, next_counter_case1) = mux(
        builder,
        at_factor,
        counter_plus_one,
        zero_counter,
        WireTy::Bits(cw),
    )?;

    // next_active_case1 = Not(at_factor).
    let (builder, next_active_case1) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(Op::Not, vec![at_factor], next_active_case1)?;

    // ---- case 2 (not active, valid_in): latch sample, start sequence ----
    // data_case2 = data_in
    // next_held_case2 = data_in
    // next_counter_case2 = case2_counter (1 if factor>1, else 0)
    // next_active_case2 = case2_active (true if factor>1, else false)

    // ---- case 3 (not active, not valid_in): idle ----
    // data_case3 = zero_sample
    // next_held_case3 = held
    // next_counter_case3 = counter
    // next_active_case3 = false (already false)

    // ---- combine cases ----
    // Two-level MUX: outer selects on `active`, inner selects on `valid_in`.

    // -- data_out --
    // inner = Mux(valid_in, zero_sample, data_in)    [case3 vs case2]
    let (builder, data_inner) = mux(builder, valid_in, zero_sample, data_in, SAMPLE_WIRE)?;
    // data_out = Mux(active, data_inner, data_case1)
    let (builder, data_out) = mux(builder, active, data_inner, data_case1, SAMPLE_WIRE)?;

    // -- valid_out = Or(active, valid_in) --
    let (builder, valid_out) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Or),
        vec![active, valid_in],
        valid_out,
    )?;

    // -- next_held --
    // inner = Mux(valid_in, held, data_in)            [case3: keep, case2: latch]
    let (builder, held_inner) = mux(builder, valid_in, held, data_in, SAMPLE_WIRE)?;
    // next_held = Mux(active, held_inner, held)       [case1: keep]
    let (builder, next_held) = mux(builder, active, held_inner, held, SAMPLE_WIRE)?;

    // -- next_counter --
    // inner = Mux(valid_in, counter, case2_counter)   [case3: keep, case2: set]
    let (builder, ctr_inner) = mux(
        builder,
        valid_in,
        counter,
        case2_counter,
        WireTy::Bits(cw),
    )?;
    // next_counter = Mux(active, ctr_inner, next_counter_case1)
    let (builder, next_counter) = mux(
        builder,
        active,
        ctr_inner,
        next_counter_case1,
        WireTy::Bits(cw),
    )?;

    // -- next_active --
    // inner = Mux(valid_in, const_false, case2_active) [case3: false, case2: set]
    let (builder, act_inner) = mux(builder, valid_in, const_false, case2_active, VALID_WIRE)?;
    // next_active = Mux(active, act_inner, next_active_case1)
    let (builder, next_active) = mux(builder, active, act_inner, next_active_case1, VALID_WIRE)?;

    let graph = builder.build();

    // ---- wire layout ----

    let state_wire_count = 3; // held, counter, active
    let input_wires = vec![held, counter, active, data_in, valid_in];
    let output_wires = vec![next_held, next_counter, next_active, data_out, valid_out];

    let initial_state = i32_to_bit_seq(0)
        .concat((0..cw).map(|_| false).collect::<BitSeq>())
        .concat(BitSeq::from_iter([false]));

    Ok(RawDspBlock::new(
        graph,
        input_wires,
        output_wires,
        initial_state,
        state_wire_count,
    ))
}

#[cfg(test)]
mod tests {
    use super::build_interpolator;

    #[test]
    fn interpolator_factor_2_has_three_state_wires() -> Result<(), crate::error::Error> {
        let block = build_interpolator(2)?;
        assert_eq!(block.state_wire_count(), 3);
        Ok(())
    }

    #[test]
    fn interpolator_factor_zero_errors() {
        assert!(build_interpolator(0).is_err());
    }

    #[test]
    fn interpolator_factor_1_is_valid() -> Result<(), crate::error::Error> {
        let block = build_interpolator(1)?;
        assert_eq!(block.state_wire_count(), 3);
        Ok(())
    }
}
