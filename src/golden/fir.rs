//! Pure FIR convolution golden model.
//!
//! Computes `output[n] = sum_{k=0}^{N-1} h[k] * x[n-k]` using
//! widening multiply to `i64` for accumulation, then truncates
//! back to `i32` by right-shifting by the specified fractional bits.

use crate::error::Error;
use crate::interpret::signal::FractionalBits;
use crate::sample::element::Sample;

/// Convolve `input` with `coefficients` (FIR filter).
///
/// Output length equals `input.len()`.  The first `coefficients.len() - 1`
/// outputs use zero-padding for missing history.
///
/// The `i64` accumulator is right-shifted by `frac_bits` to truncate
/// back to `i32`.  For integer coefficients, pass
/// `FractionalBits::new(0)`.
///
/// # Errors
///
/// Returns [`Error::Fir`] if `coefficients` is empty.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::fir::fir_convolve;
/// use dsp_cat::interpret::signal::FractionalBits;
/// use dsp_cat::sample::element::Sample;
///
/// let input = vec![Sample::new(1), Sample::new(0), Sample::new(0)];
/// let coeffs = vec![Sample::new(3), Sample::new(2), Sample::new(1)];
/// let output = fir_convolve(&input, &coeffs, FractionalBits::new(0)).ok();
/// let values: Option<Vec<i32>> = output.map(|v| v.iter().map(|s| s.value()).collect());
/// // Impulse response = coefficients
/// assert_eq!(values, Some(vec![3, 2, 1]));
/// ```
pub fn fir_convolve(
    input: &[Sample],
    coefficients: &[Sample],
    frac_bits: FractionalBits,
) -> Result<Vec<Sample>, Error> {
    if coefficients.is_empty() {
        Err(Error::Fir("empty coefficient set".to_owned()))
    } else {
        let shift = frac_bits.value();
        Ok((0..input.len())
            .map(|n| {
                let acc = coefficients.iter().enumerate().fold(0_i64, |acc, (k, coeff)| {
                    let sample = n
                        .checked_sub(k)
                        .and_then(|idx| input.get(idx))
                        .copied()
                        .unwrap_or(Sample::ZERO);
                    acc.saturating_add(sample.widening_mul(*coeff))
                });
                let shifted = acc >> shift;
                Sample::new(
                    i32::try_from(shifted.clamp(i64::from(i32::MIN), i64::from(i32::MAX)))
                        .unwrap_or(i32::MAX),
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_coefficients_is_error() {
        let result = fir_convolve(&[Sample::new(1)], &[], FractionalBits::new(0));
        assert!(result.is_err());
    }

    #[test]
    fn identity_filter_is_passthrough() -> Result<(), Error> {
        let input = vec![Sample::new(10), Sample::new(20), Sample::new(30)];
        let coeffs = vec![Sample::new(1)];
        let output = fir_convolve(&input, &coeffs, FractionalBits::new(0))?;
        assert_eq!(output, input);
        Ok(())
    }

    #[test]
    fn impulse_response_equals_coefficients() -> Result<(), Error> {
        let impulse = vec![Sample::new(1), Sample::ZERO, Sample::ZERO, Sample::ZERO];
        let coeffs = vec![Sample::new(5), Sample::new(3), Sample::new(1)];
        let output = fir_convolve(&impulse, &coeffs, FractionalBits::new(0))?;
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![5, 3, 1, 0]);
        Ok(())
    }

    #[test]
    fn three_tap_moving_average() -> Result<(), Error> {
        let input: Vec<Sample> = vec![3, 6, 9, 12, 15].into_iter().map(Sample::new).collect();
        let coeffs = vec![Sample::new(1), Sample::new(1), Sample::new(1)];
        let output = fir_convolve(&input, &coeffs, FractionalBits::new(0))?;
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        // [3, 3+6, 3+6+9, 6+9+12, 9+12+15]
        assert_eq!(values, vec![3, 9, 18, 27, 36]);
        Ok(())
    }

    #[test]
    fn empty_input_produces_empty() -> Result<(), Error> {
        let output = fir_convolve(&[], &[Sample::new(1)], FractionalBits::new(0))?;
        assert!(output.is_empty());
        Ok(())
    }
}
