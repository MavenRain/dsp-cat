//! Pure decimation golden model.

use crate::error::Error;
use crate::interpret::signal::RateFactor;
use crate::sample::element::Sample;

/// Keep every `factor`-th sample, starting from sample 0.
///
/// # Errors
///
/// Returns [`Error::InvalidRateFactor`] if the factor is zero.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::decimator::decimate;
/// use dsp_cat::interpret::signal::RateFactor;
/// use dsp_cat::sample::element::Sample;
///
/// let input: Vec<Sample> = (0..8).map(Sample::new).collect();
/// let output = decimate(&input, RateFactor::new(2)).ok();
/// let values: Option<Vec<i32>> = output.map(|v| v.iter().map(|s| s.value()).collect());
/// assert_eq!(values, Some(vec![0, 2, 4, 6]));
/// ```
pub fn decimate(input: &[Sample], factor: RateFactor) -> Result<Vec<Sample>, Error> {
    if factor.value() == 0 {
        Err(Error::InvalidRateFactor {
            factor: factor.value(),
        })
    } else {
        Ok(input
            .iter()
            .step_by(factor.value())
            .copied()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_one_is_identity() -> Result<(), Error> {
        let input = vec![Sample::new(1), Sample::new(2), Sample::new(3)];
        let output = decimate(&input, RateFactor::new(1))?;
        assert_eq!(output, input);
        Ok(())
    }

    #[test]
    fn factor_two_keeps_evens() -> Result<(), Error> {
        let input: Vec<Sample> = (0..6).map(Sample::new).collect();
        let output = decimate(&input, RateFactor::new(2))?;
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![0, 2, 4]);
        Ok(())
    }

    #[test]
    fn factor_zero_is_error() {
        let result = decimate(&[Sample::new(1)], RateFactor::new(0));
        assert!(result.is_err());
    }

    #[test]
    fn empty_input_produces_empty() -> Result<(), Error> {
        let output = decimate(&[], RateFactor::new(4))?;
        assert!(output.is_empty());
        Ok(())
    }
}
