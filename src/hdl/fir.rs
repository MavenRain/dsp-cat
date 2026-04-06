//! FIR filter DSP block.
//!
//! Direct-form FIR with a shift-register history buffer and
//! unrolled multiply-accumulate chain.  The entire convolution
//! is computed combinationally in one graph evaluation.

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op, WireTy};
use hdl_cat::kind::BitSeq;

use crate::error::Error;
use crate::hdl::common::{
    arith_shr, i32_to_bit_seq, mux, sign_extend, truncate, zero_sample_init,
    SAMPLE_WIRE, VALID_WIRE,
};
use crate::hdl::raw::RawDspBlock;

/// Build a FIR filter from coefficients and fractional shift.
///
/// State: `tap_count` sample registers (`Signed(32)` each) forming
/// a shift-register history buffer.
///
/// At each cycle with `valid_in`:
/// - Shifts new sample into the history.
/// - Computes `sum(history[k] * coeff[k])` with 64-bit accumulation.
/// - Outputs `truncate_32(sum >>> shift)`.
///
/// When `!valid_in`: outputs zero with `valid_out = false`,
/// history unchanged.
///
/// # Errors
///
/// Returns [`Error::Fir`] if `coefficients` is empty.
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn build_fir(coefficients: &[i32], shift: u32) -> Result<RawDspBlock, Error> {
    if coefficients.is_empty() {
        return Err(Error::Fir("empty coefficients".into()));
    }

    let tap_count = coefficients.len();

    // ---- state wires: shift register ----

    let (builder, regs) = (0..tap_count).try_fold(
        (HdlGraphBuilder::new(), Vec::with_capacity(tap_count)),
        |(b, mut v), _| {
            let (b, w) = b.with_wire(SAMPLE_WIRE);
            v.push(w);
            Ok::<_, Error>((b, v))
        },
    )?;

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

    // 64-bit zero for initial accumulator.
    let (builder, zero_64) = builder.with_wire(WireTy::Signed(64));
    let builder = builder.with_instruction(
        Op::Const {
            bits: (0..64).map(|_| false).collect::<BitSeq>(),
            ty: WireTy::Signed(64),
        },
        vec![],
        zero_64,
    )?;

    // ---- unrolled multiply-accumulate chain ----
    // Tap sources: tap[0] = data_in, tap[k] = regs[k-1] for k > 0.
    // This mirrors the circular buffer write-then-read: the newest
    // sample is data_in (just written), then regs[0] is the second
    // newest, etc.

    let (builder, acc) = (0..tap_count).try_fold(
        (builder, zero_64),
        |(builder, acc), k| {
            let tap_wire = if k == 0 { data_in } else { regs[k - 1] };

            // Sign-extend tap to 64 bits.
            let (builder, tap_ext) = sign_extend(builder, tap_wire, 32, 64)?;

            // Coefficient constant (sign-extended to 64).
            let coeff_val = coefficients[k];
            let (builder, coeff_wire) = builder.with_wire(SAMPLE_WIRE);
            let builder = builder.with_instruction(
                Op::Const {
                    bits: i32_to_bit_seq(coeff_val),
                    ty: SAMPLE_WIRE,
                },
                vec![],
                coeff_wire,
            )?;
            let (builder, coeff_ext) = sign_extend(builder, coeff_wire, 32, 64)?;

            // product = tap * coeff (64-bit wrapping).
            let (builder, product) = builder.with_wire(WireTy::Signed(64));
            let builder = builder.with_instruction(
                Op::Bin(BinOp::Mul),
                vec![tap_ext, coeff_ext],
                product,
            )?;

            // acc = acc + product (64-bit wrapping).
            let (builder, new_acc) = builder.with_wire(WireTy::Signed(64));
            let builder = builder.with_instruction(
                Op::Bin(BinOp::Add),
                vec![acc, product],
                new_acc,
            )?;

            Ok::<_, Error>((builder, new_acc))
        },
    )?;

    // ---- shift and truncate ----

    let (builder, shifted) = arith_shr(builder, acc, shift, 64)?;
    let (builder, result) = truncate(builder, shifted, 32)?;

    // ---- output gated by valid_in ----

    let (builder, data_out) = mux(builder, valid_in, zero_sample, result, SAMPLE_WIRE)?;

    // ---- shift register next-state ----
    // next_regs[0] = Mux(valid_in, regs[0], data_in)
    // next_regs[k] = Mux(valid_in, regs[k], regs[k-1])

    let (builder, next_regs) = (0..tap_count).try_fold(
        (builder, Vec::with_capacity(tap_count)),
        |(b, mut v), k| {
            let source = if k == 0 { data_in } else { regs[k - 1] };
            let (b, nr) = mux(b, valid_in, regs[k], source, SAMPLE_WIRE)?;
            v.push(nr);
            Ok::<_, Error>((b, v))
        },
    )?;

    let graph = builder.build();

    // ---- assemble wires ----

    let state_wire_count = tap_count;

    let mut input_wires = Vec::with_capacity(state_wire_count + 2);
    input_wires.extend(regs.iter().copied());
    input_wires.push(data_in);
    input_wires.push(valid_in);

    let mut output_wires = Vec::with_capacity(state_wire_count + 2);
    output_wires.extend(next_regs.iter().copied());
    output_wires.push(data_out);
    output_wires.push(valid_in); // valid_out = valid_in (combinational)

    let initial_state = (0..tap_count).fold(BitSeq::new(), |acc, _| {
        acc.concat(zero_sample_init())
    });

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
    use super::build_fir;

    #[test]
    fn fir_single_tap_has_one_state_wire() -> Result<(), crate::error::Error> {
        let block = build_fir(&[1], 0)?;
        assert_eq!(block.state_wire_count(), 1);
        Ok(())
    }

    #[test]
    fn fir_three_taps_has_three_state_wires() -> Result<(), crate::error::Error> {
        let block = build_fir(&[3, 2, 1], 0)?;
        assert_eq!(block.state_wire_count(), 3);
        Ok(())
    }

    #[test]
    fn fir_empty_coefficients_errors() {
        assert!(build_fir(&[], 0).is_err());
    }

    #[test]
    fn fir_initial_state_width_matches_taps() -> Result<(), crate::error::Error> {
        let block = build_fir(&[1, 2, 3, 4], 15)?;
        // 4 taps * 32 bits = 128.
        assert_eq!(block.initial_state().len(), 128);
        Ok(())
    }
}
