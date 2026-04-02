//! Pure interpolation golden model.

use crate::error::Error;
use crate::interpret::signal::RateFactor;
use crate::sample::element::Sample;

/// Insert `factor - 1` zero-valued samples between each input sample.
///
/// # Errors
///
/// Returns [`Error::InvalidRateFactor`] if the factor is zero.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::interpolator::interpolate;
/// use dsp_cat::interpret::signal::RateFactor;
/// use dsp_cat::sample::element::Sample;
///
/// let input = vec![Sample::new(1), Sample::new(2)];
/// let output = interpolate(&input, RateFactor::new(3)).ok();
/// let values: Option<Vec<i32>> = output.map(|v| v.iter().map(|s| s.value()).collect());
/// assert_eq!(values, Some(vec![1, 0, 0, 2, 0, 0]));
/// ```
pub fn interpolate(input: &[Sample], factor: RateFactor) -> Result<Vec<Sample>, Error> {
    if factor.value() == 0 {
        Err(Error::InvalidRateFactor {
            factor: factor.value(),
        })
    } else {
        Ok(input
            .iter()
            .flat_map(|s| {
                std::iter::once(*s).chain(std::iter::repeat_n(Sample::ZERO, factor.value() - 1))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_one_is_identity() -> Result<(), Error> {
        let input = vec![Sample::new(1), Sample::new(2)];
        let output = interpolate(&input, RateFactor::new(1))?;
        assert_eq!(output, input);
        Ok(())
    }

    #[test]
    fn factor_two_inserts_one_zero() -> Result<(), Error> {
        let input = vec![Sample::new(10), Sample::new(20)];
        let output = interpolate(&input, RateFactor::new(2))?;
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![10, 0, 20, 0]);
        Ok(())
    }

    #[test]
    fn factor_zero_is_error() {
        let result = interpolate(&[Sample::new(1)], RateFactor::new(0));
        assert!(result.is_err());
    }

    #[test]
    fn empty_input_produces_empty() -> Result<(), Error> {
        let output = interpolate(&[], RateFactor::new(4))?;
        assert!(output.is_empty());
        Ok(())
    }
}
