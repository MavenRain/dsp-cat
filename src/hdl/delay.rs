//! Delay line DSP block.
//!
//! A shift-register chain that delays the input signal by a fixed
//! number of sample periods.

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op, WireTy};
use hdl_cat::kind::BitSeq;

use crate::error::Error;
use crate::hdl::common::{
    bits_for_value, const_unsigned, i32_to_bit_seq, mux, zero_sample_init,
    DSP_IO_WIRE_COUNT, SAMPLE_WIRE, VALID_WIRE,
};
use crate::hdl::raw::RawDspBlock;

/// Build a delay line with the given `depth`.
///
/// State: `depth` sample registers (`Signed(32)` each) + fill counter.
/// Latency: `depth` sample periods.
///
/// The first `depth` valid inputs fill the register chain.  After
/// that, each valid input shifts in a new sample and produces the
/// oldest sample as output.  Invalid cycles hold state and output
/// zero with `valid_out = false`.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn build_delay(depth: usize) -> Result<RawDspBlock, Error> {
    let depth = depth.max(1);
    let fw = bits_for_value(depth);

    // ---- declare state wires ----

    let builder = HdlGraphBuilder::new();

    // depth registers for the shift chain.
    let (builder, regs) = (0..depth).try_fold(
        (builder, Vec::with_capacity(depth)),
        |(b, mut v), _| {
            let (b, w) = b.with_wire(SAMPLE_WIRE);
            v.push(w);
            Ok::<_, Error>((b, v))
        },
    )?;

    // Fill counter.
    let (builder, fill_ctr) = builder.with_wire(WireTy::Bits(fw));

    // ---- declare input wires ----

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
    let (builder, fill_one) = const_unsigned(builder, 1, fw)?;
    let (builder, depth_const) = const_unsigned(builder, u128::try_from(depth).unwrap_or(0), fw)?;

    // ---- shift register next-state ----
    // next_reg[0] = Mux(valid_in, reg[0], data_in)
    // next_reg[i] = Mux(valid_in, reg[i], reg[i-1])  for i > 0

    let (builder, next_regs) = (0..depth).try_fold(
        (builder, Vec::with_capacity(depth)),
        |(b, mut v), i| {
            let source = if i == 0 { data_in } else { regs[i - 1] };
            let (b, nr) = mux(b, valid_in, regs[i], source, SAMPLE_WIRE)?;
            v.push(nr);
            Ok::<_, Error>((b, v))
        },
    )?;

    // ---- fill counter logic ----

    // is_full = Eq(fill_ctr, depth_const).
    let (builder, is_full) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Eq),
        vec![fill_ctr, depth_const],
        is_full,
    )?;

    // fill_plus_one = fill_ctr + 1.
    let (builder, fill_plus_one) = builder.with_wire(WireTy::Bits(fw));
    let builder = builder.with_instruction(
        Op::Bin(BinOp::Add),
        vec![fill_ctr, fill_one],
        fill_plus_one,
    )?;

    // fill_update = Mux(is_full, fill_plus_one, fill_ctr).
    // If full, keep at depth; else increment.
    let (builder, fill_update) = mux(builder, is_full, fill_plus_one, fill_ctr, WireTy::Bits(fw))?;

    // next_fill = Mux(valid_in, fill_ctr, fill_update).
    // Only advance when valid_in.
    let (builder, next_fill) = mux(builder, valid_in, fill_ctr, fill_update, WireTy::Bits(fw))?;

    // ---- output ----

    // data_out = Mux(valid_in, zero_sample, reg[depth-1]).
    let (builder, data_out) = mux(
        builder,
        valid_in,
        zero_sample,
        regs[depth - 1],
        SAMPLE_WIRE,
    )?;

    // valid_out = And(valid_in, is_full).
    let (builder, valid_out) = builder.with_wire(VALID_WIRE);
    let builder = builder.with_instruction(
        Op::Bin(BinOp::And),
        vec![valid_in, is_full],
        valid_out,
    )?;

    let graph = builder.build();

    // ---- assemble wires ----

    let state_wire_count = depth + 1; // depth regs + fill counter

    let mut input_wires = Vec::with_capacity(state_wire_count + DSP_IO_WIRE_COUNT);
    input_wires.extend(regs.iter().copied());
    input_wires.push(fill_ctr);
    input_wires.push(data_in);
    input_wires.push(valid_in);

    let mut output_wires = Vec::with_capacity(state_wire_count + DSP_IO_WIRE_COUNT);
    output_wires.extend(next_regs.iter().copied());
    output_wires.push(next_fill);
    output_wires.push(data_out);
    output_wires.push(valid_out);

    // Initial state: all registers zero, fill counter zero.
    let initial_state = (0..depth)
        .fold(BitSeq::new(), |acc, _| acc.concat(zero_sample_init()))
        .concat((0..fw).map(|_| false).collect::<BitSeq>());

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
    use super::build_delay;

    #[test]
    fn delay_depth_1_has_two_state_wires() -> Result<(), crate::error::Error> {
        let block = build_delay(1)?;
        // 1 register + 1 fill counter.
        assert_eq!(block.state_wire_count(), 2);
        Ok(())
    }

    #[test]
    fn delay_depth_4_has_five_state_wires() -> Result<(), crate::error::Error> {
        let block = build_delay(4)?;
        // 4 registers + 1 fill counter.
        assert_eq!(block.state_wire_count(), 5);
        Ok(())
    }

    #[test]
    fn delay_depth_0_treated_as_1() -> Result<(), crate::error::Error> {
        let block = build_delay(0)?;
        assert_eq!(block.state_wire_count(), 2);
        Ok(())
    }
}
