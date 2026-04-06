//! Shared types and IR helper functions for DSP block construction.
//!
//! Every DSP block in this module shares the same I/O interface:
//! a [`SignedBits<32>`](hdl_cat::bits::SignedBits) data wire paired
//! with a `bool` valid wire.

use hdl_cat::ir::{BinOp, HdlGraphBuilder, Op, WireId, WireTy};
use hdl_cat::kind::BitSeq;

use crate::error::Error;

/// The width of a single DSP sample in bits.
pub const SAMPLE_WIDTH: u32 = 32;

/// The wire type for a DSP sample (signed 32-bit).
pub const SAMPLE_WIRE: WireTy = WireTy::Signed(SAMPLE_WIDTH);

/// The wire type for a valid flag (single bit).
pub const VALID_WIRE: WireTy = WireTy::Bit;

/// Number of data wires in the standard DSP I/O interface
/// (one sample wire + one valid wire).
pub const DSP_IO_WIRE_COUNT: usize = 2;

// -- BitSeq construction helpers ------------------------------------------

/// Encode an `i32` as a 32-bit LSB-first [`BitSeq`].
pub fn i32_to_bit_seq(value: i32) -> BitSeq {
    let bits = u32::from_ne_bytes(value.to_ne_bytes());
    (0..32).map(|i| (bits >> i) & 1 == 1).collect()
}

/// Decode a 32-bit LSB-first [`BitSeq`] back to `i32`.
///
/// # Errors
///
/// Returns [`Error::Sample`] if the sequence length is not 32.
pub fn bit_seq_to_i32(seq: &BitSeq) -> Result<i32, Error> {
    (seq.len() == 32)
        .then(|| {
            let unsigned = (0..32).fold(0u32, |acc, i| {
                acc | (u32::from(seq.bit(i)) << i)
            });
            i32::from_ne_bytes(unsigned.to_ne_bytes())
        })
        .ok_or_else(|| {
            Error::Sample(format!(
                "expected 32-bit sequence, got {} bits",
                seq.len()
            ))
        })
}

/// A zero-valued 32-bit initial state suitable for a sample register.
pub fn zero_sample_init() -> BitSeq {
    i32_to_bit_seq(0)
}

/// A zero-valued 1-bit initial state suitable for a valid register.
pub fn zero_valid_init() -> BitSeq {
    BitSeq::from_iter([false])
}

/// Minimum number of bits to represent `value`.
///
/// Returns at least 1 (a single-bit bus), even for `value == 0`.
#[must_use]
pub fn bits_for_value(value: usize) -> u32 {
    if value == 0 {
        1
    } else {
        usize::BITS - value.leading_zeros()
    }
}

// -- IR builder helpers ---------------------------------------------------

/// Insert a constant `i32` value as a `Signed(32)` wire.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if the instruction cannot be added.
pub fn const_signed_32(
    builder: HdlGraphBuilder,
    value: i32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (builder, wire) = builder.with_wire(WireTy::Signed(32));
    let builder = builder.with_instruction(
        Op::Const {
            bits: i32_to_bit_seq(value),
            ty: WireTy::Signed(32),
        },
        vec![],
        wire,
    )?;
    Ok((builder, wire))
}

/// Insert a constant unsigned value as a `Bits(width)` wire.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if the instruction cannot be added.
pub fn const_unsigned(
    builder: HdlGraphBuilder,
    value: u128,
    width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (builder, wire) = builder.with_wire(WireTy::Bits(width));
    let seq: BitSeq = (0..width)
        .map(|i| (value >> i) & 1 == 1)
        .collect();
    let builder = builder.with_instruction(
        Op::Const {
            bits: seq,
            ty: WireTy::Bits(width),
        },
        vec![],
        wire,
    )?;
    Ok((builder, wire))
}

/// Insert a constant bit (bool) wire.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if the instruction cannot be added.
pub fn const_bit(
    builder: HdlGraphBuilder,
    value: bool,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (builder, wire) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Const {
            bits: BitSeq::from_iter([value]),
            ty: WireTy::Bit,
        },
        vec![],
        wire,
    )?;
    Ok((builder, wire))
}

/// Insert a 2:1 MUX instruction.
///
/// Returns the output wire.  When `selector` is true, the result
/// is `true_arm`; otherwise `false_arm`.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if the instruction cannot be added.
pub fn mux(
    builder: HdlGraphBuilder,
    selector: WireId,
    false_arm: WireId,
    true_arm: WireId,
    out_ty: WireTy,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (builder, out) = builder.with_wire(out_ty);
    let builder = builder.with_instruction(
        Op::Mux,
        vec![selector, false_arm, true_arm],
        out,
    )?;
    Ok((builder, out))
}

/// Insert a binary operation instruction.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if the instruction cannot be added.
pub fn bin_op(
    builder: HdlGraphBuilder,
    op: BinOp,
    lhs: WireId,
    rhs: WireId,
    out_ty: WireTy,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (builder, out) = builder.with_wire(out_ty);
    let builder = builder.with_instruction(
        Op::Bin(op),
        vec![lhs, rhs],
        out,
    )?;
    Ok((builder, out))
}

/// Sign-extend a `Signed(from_width)` wire to `Signed(to_width)`.
///
/// Extracts the sign bit, replicates it into a fill bus, and
/// concatenates: `result = [original_bits, sign_fill]`.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn sign_extend(
    builder: HdlGraphBuilder,
    wire: WireId,
    from_width: u32,
    to_width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    if from_width >= to_width {
        return Ok((builder, wire));
    }
    let fill_width = to_width - from_width;

    // Extract sign bit (bit from_width - 1).
    let (builder, sign_bit) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Slice {
            lo: from_width - 1,
            hi: from_width,
        },
        vec![wire],
        sign_bit,
    )?;

    // Create fill_width-bit constants for sign extension.
    let ones_bits: BitSeq = (0..fill_width).map(|_| true).collect();
    let zeros_bits: BitSeq = (0..fill_width).map(|_| false).collect();
    let (builder, ones) = builder.with_wire(WireTy::Bits(fill_width));
    let builder = builder.with_instruction(
        Op::Const {
            bits: ones_bits,
            ty: WireTy::Bits(fill_width),
        },
        vec![],
        ones,
    )?;
    let (builder, zeros) = builder.with_wire(WireTy::Bits(fill_width));
    let builder = builder.with_instruction(
        Op::Const {
            bits: zeros_bits,
            ty: WireTy::Bits(fill_width),
        },
        vec![],
        zeros,
    )?;

    // MUX: if sign_bit then ones else zeros.
    let (builder, fill) = builder.with_wire(WireTy::Bits(fill_width));
    let builder = builder.with_instruction(
        Op::Mux,
        vec![sign_bit, zeros, ones],
        fill,
    )?;

    // Concat: low = original, high = fill.
    let (builder, extended) = builder.with_wire(WireTy::Signed(to_width));
    let builder = builder.with_instruction(
        Op::Concat {
            low_width: from_width,
            high_width: fill_width,
        },
        vec![wire, fill],
        extended,
    )?;

    Ok((builder, extended))
}

/// Arithmetic right-shift a `Signed(width)` wire by a constant amount.
///
/// Implements `x >>> shift` by slicing out the upper bits and
/// concatenating sign-extension fill.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn arith_shr(
    builder: HdlGraphBuilder,
    wire: WireId,
    shift: u32,
    width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    if shift == 0 {
        return Ok((builder, wire));
    }
    if shift >= width {
        // Result is all sign bits.
        let (builder, sign_bit) = builder.with_wire(WireTy::Bit);
        let builder = builder.with_instruction(
            Op::Slice {
                lo: width - 1,
                hi: width,
            },
            vec![wire],
            sign_bit,
        )?;
        let ones_bits: BitSeq = (0..width).map(|_| true).collect();
        let zeros_bits: BitSeq = (0..width).map(|_| false).collect();
        let (builder, ones) = builder.with_wire(WireTy::Signed(width));
        let builder = builder.with_instruction(
            Op::Const {
                bits: ones_bits,
                ty: WireTy::Signed(width),
            },
            vec![],
            ones,
        )?;
        let (builder, zeros) = builder.with_wire(WireTy::Signed(width));
        let builder = builder.with_instruction(
            Op::Const {
                bits: zeros_bits,
                ty: WireTy::Signed(width),
            },
            vec![],
            zeros,
        )?;
        return mux(builder, sign_bit, zeros, ones, WireTy::Signed(width));
    }

    let kept = width - shift;

    // Slice bits [shift..width) from the input.
    let (builder, sliced) = builder.with_wire(WireTy::Bits(kept));
    let builder = builder.with_instruction(
        Op::Slice { lo: shift, hi: width },
        vec![wire],
        sliced,
    )?;

    // Extract sign bit.
    let (builder, sign_bit) = builder.with_wire(WireTy::Bit);
    let builder = builder.with_instruction(
        Op::Slice {
            lo: width - 1,
            hi: width,
        },
        vec![wire],
        sign_bit,
    )?;

    // Fill bits for sign extension.
    let ones_bits: BitSeq = (0..shift).map(|_| true).collect();
    let zeros_bits: BitSeq = (0..shift).map(|_| false).collect();
    let (builder, ones) = builder.with_wire(WireTy::Bits(shift));
    let builder = builder.with_instruction(
        Op::Const {
            bits: ones_bits,
            ty: WireTy::Bits(shift),
        },
        vec![],
        ones,
    )?;
    let (builder, zeros) = builder.with_wire(WireTy::Bits(shift));
    let builder = builder.with_instruction(
        Op::Const {
            bits: zeros_bits,
            ty: WireTy::Bits(shift),
        },
        vec![],
        zeros,
    )?;
    let (builder, fill) = builder.with_wire(WireTy::Bits(shift));
    let builder = builder.with_instruction(
        Op::Mux,
        vec![sign_bit, zeros, ones],
        fill,
    )?;

    // Concat: low = sliced, high = fill.
    let (builder, result) = builder.with_wire(WireTy::Signed(width));
    let builder = builder.with_instruction(
        Op::Concat {
            low_width: kept,
            high_width: shift,
        },
        vec![sliced, fill],
        result,
    )?;

    Ok((builder, result))
}

/// Truncate a wide signed wire to `Signed(to_width)` by slicing
/// the low bits.
///
/// # Errors
///
/// Returns [`Error::Hdl`] if graph construction fails.
pub fn truncate(
    builder: HdlGraphBuilder,
    wire: WireId,
    to_width: u32,
) -> Result<(HdlGraphBuilder, WireId), Error> {
    let (builder, out) = builder.with_wire(WireTy::Signed(to_width));
    let builder = builder.with_instruction(
        Op::Slice { lo: 0, hi: to_width },
        vec![wire],
        out,
    )?;
    Ok((builder, out))
}
