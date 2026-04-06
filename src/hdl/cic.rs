//! CIC (Cascaded Integrator-Comb) decimation filter DSP block.
//!
//! Chains M integrator stages, a decimator, and M comb stages.
//! No multipliers required; all arithmetic is wrapping add/sub.

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op, WireTy};
use hdl_cat::kind::BitSeq;

use crate::error::Error;
use crate::hdl::common::{
    bits_for_value, const_unsigned, i32_to_bit_seq, mux, zero_sample_init,
    SAMPLE_WIRE, VALID_WIRE,
};
use crate::hdl::raw::RawDspBlock;

/// Build a CIC decimation filter.
///
/// State: M integrator accumulators + decimation counter + M comb
/// previous values.
///
/// The integrators run on every valid input.  The decimator passes
/// every `rate_factor`-th integrator output to the comb chain.
/// Comb previous values update only on decimated samples.
///
/// # Errors
///
/// Returns [`Error::Cic`] if `order` is zero.
/// Returns [`Error::InvalidRateFactor`] if `rate_factor` is zero.
/// Returns [`Error::Hdl`] if graph construction fails.
#[allow(clippy::too_many_lines)]
pub fn build_cic(order: usize, rate_factor: usize) -> Result<RawDspBlock, Error> {
    if order == 0 {
        return Err(Error::Cic("order must be at least 1".into()));
    }
    if rate_factor == 0 {
        return Err(Error::InvalidRateFactor { factor: rate_factor });
    }

    let cw = bits_for_value(rate_factor);

    // ---- state wires ----

    // M integrator accumulators.
    let (builder, integs) = (0..order).try_fold(
        (HdlGraphBuilder::new(), Vec::with_capacity(order)),
        |(b, mut v), _| {
            let (b, w) = b.with_wire(SAMPLE_WIRE);
            v.push(w);
            Ok::<_, Error>((b, v))
        },
    )?;

    // Decimation counter.
    let (builder, counter) = builder.with_wire(WireTy::Bits(cw));

    // M comb previous values.
    let (builder, comb_prevs) = (0..order).try_fold(
        (builder, Vec::with_capacity(order)),
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
    let (builder, zero_ctr) = const_unsigned(builder, 0, cw)?;
    let (builder, one_ctr) = const_unsigned(builder, 1, cw)?;
    let (builder, factor_const) = const_unsigned(
        builder,
        u128::try_from(rate_factor).unwrap_or(0),
        cw,
    )?;

    // ---- integrator chain (combinational) ----
    // integ_out[0] = integs[0] + data_in
    // integ_out[k] = integs[k] + integ_out[k-1]

    let (builder, integ_outs) = (0..order).try_fold(
        (builder, Vec::with_capacity(order)),
        |(b, mut v), k| {
            let prev_out = if k == 0 { data_in } else { v[k - 1] };
            let (b, out) = b.with_wire(SAMPLE_WIRE);
            let b = b.with_instruction(
                Op::Bin(BinOp::Add),
                vec![integs[k], prev_out],
                out,
            )?;
            v.push(out);
            Ok::<_, Error>((b, v))
        },
    )?;

    // next_integ[k] = Mux(valid_in, integs[k], integ_outs[k]).
    let (builder, next_integs) = (0..order).try_fold(
        (builder, Vec::with_capacity(order)),
        |(b, mut v), k| {
            let (b, ni) = mux(b, valid_in, integs[k], integ_outs[k], SAMPLE_WIRE)?;
            v.push(ni);
            Ok::<_, Error>((b, v))
        },
    )?;

    // ---- decimator logic ----

    let (builder, decim_pass) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Eq),
        vec![counter, zero_ctr],
        decim_pass,
    )?;

    let (builder, ctr_plus_one) = builder.with_wire(WireTy::Bits(cw));
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Add),
        vec![counter, one_ctr],
        ctr_plus_one,
    )?;

    let (builder, at_factor) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Eq),
        vec![ctr_plus_one, factor_const],
        at_factor,
    )?;

    let (builder, wrapped_ctr) = mux(builder, at_factor, ctr_plus_one, zero_ctr, WireTy::Bits(cw))?;
    let (builder, next_counter) = mux(builder, valid_in, counter, wrapped_ctr, WireTy::Bits(cw))?;

    // valid_and_decim = And(valid_in, decim_pass).
    let (builder, valid_and_decim) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::And),
        vec![valid_in, decim_pass],
        valid_and_decim,
    )?;

    // ---- comb chain (combinational) ----
    // comb_val[0] = integ_outs[M-1] - comb_prevs[0]
    // comb_val[k] = comb_val[k-1] - comb_prevs[k]
    // comb_input[0] = integ_outs[M-1]
    // comb_input[k] = comb_val[k-1]

    let integ_final = integ_outs[order - 1];

    let (builder, comb_vals, comb_inputs) = (0..order).try_fold(
        (builder, Vec::with_capacity(order), Vec::with_capacity(order)),
        |(b, mut vals, mut inputs), k| {
            let input_to_comb = if k == 0 { integ_final } else { vals[k - 1] };
            inputs.push(input_to_comb);
            let (b, diff) = b.with_wire(SAMPLE_WIRE);
            let b = b.with_instruction(
                Op::Bin(BinOp::Sub),
                vec![input_to_comb, comb_prevs[k]],
                diff,
            )?;
            vals.push(diff);
            Ok::<_, Error>((b, vals, inputs))
        },
    )?;

    // next_comb_prevs[k] = Mux(valid_and_decim, comb_prevs[k], comb_inputs[k]).
    let (builder, next_comb_prevs) = (0..order).try_fold(
        (builder, Vec::with_capacity(order)),
        |(b, mut v), k| {
            let (b, nc) = mux(
                b,
                valid_and_decim,
                comb_prevs[k],
                comb_inputs[k],
                SAMPLE_WIRE,
            )?;
            v.push(nc);
            Ok::<_, Error>((b, v))
        },
    )?;

    // ---- output ----

    let comb_final = comb_vals[order - 1];
    let (builder, data_out) = mux(builder, valid_and_decim, zero_sample, comb_final, SAMPLE_WIRE)?;

    let graph = builder.build();

    // ---- assemble wires ----

    let state_wire_count = order + 1 + order; // integs + counter + comb_prevs

    let mut input_wires = Vec::with_capacity(state_wire_count + 2);
    input_wires.extend(integs.iter().copied());
    input_wires.push(counter);
    input_wires.extend(comb_prevs.iter().copied());
    input_wires.push(data_in);
    input_wires.push(valid_in);

    let mut output_wires = Vec::with_capacity(state_wire_count + 2);
    output_wires.extend(next_integs.iter().copied());
    output_wires.push(next_counter);
    output_wires.extend(next_comb_prevs.iter().copied());
    output_wires.push(data_out);
    output_wires.push(valid_and_decim); // valid_out

    // Initial state: all zeros.
    let initial_state = (0..order)
        .fold(BitSeq::new(), |acc, _| acc.concat(zero_sample_init()))
        .concat((0..cw).map(|_| false).collect::<BitSeq>())
        .concat(
            (0..order).fold(BitSeq::new(), |acc, _| acc.concat(zero_sample_init())),
        );

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
    use super::build_cic;

    #[test]
    fn cic_order_1_rate_2() -> Result<(), crate::error::Error> {
        let block = build_cic(1, 2)?;
        // 1 integrator + 1 counter + 1 comb = 3 state wires.
        assert_eq!(block.state_wire_count(), 3);
        Ok(())
    }

    #[test]
    fn cic_order_3_rate_4() -> Result<(), crate::error::Error> {
        let block = build_cic(3, 4)?;
        // 3 integrators + 1 counter + 3 combs = 7 state wires.
        assert_eq!(block.state_wire_count(), 7);
        Ok(())
    }

    #[test]
    fn cic_order_zero_errors() {
        assert!(build_cic(0, 2).is_err());
    }

    #[test]
    fn cic_rate_zero_errors() {
        assert!(build_cic(1, 0).is_err());
    }
}
