//! Pure CIC (Cascaded Integrator-Comb) golden model.
//!
//! A CIC decimation filter of order M with rate change factor R:
//!
//! 1. M cascaded integrator stages at the input rate (prefix sums).
//! 2. Decimation by R (keep every Rth sample).
//! 3. M cascaded comb stages at the output rate (first differences).

use crate::error::Error;
use crate::interpret::signal::{CicOrder, RateFactor};
use crate::sample::element::Sample;

/// CIC decimation filter.
///
/// Bit growth is `order * ceil(log2(rate_factor))` bits.  The caller
/// must ensure the accumulator width is sufficient.
///
/// # Errors
///
/// Returns [`Error::Cic`] if order is zero.
/// Returns [`Error::InvalidRateFactor`] if rate factor is zero.
///
/// # Examples
///
/// ```
/// use dsp_cat::golden::cic::cic_decimate;
/// use dsp_cat::interpret::signal::{CicOrder, RateFactor};
/// use dsp_cat::sample::element::Sample;
///
/// let input: Vec<Sample> = (1..=8).map(Sample::new).collect();
/// let output = cic_decimate(&input, CicOrder::new(1), RateFactor::new(2)).ok();
/// let values: Option<Vec<i32>> = output.map(|v| v.iter().map(|s| s.value()).collect());
/// // Order-1 CIC decimate-by-2: integrate -> decimate -> comb
/// // Integrate: [1, 3, 6, 10, 15, 21, 28, 36]
/// // Decimate by 2: [1, 6, 15, 28]
/// // Comb (first diff): [1, 5, 9, 13]
/// assert_eq!(values, Some(vec![1, 5, 9, 13]));
/// ```
pub fn cic_decimate(
    input: &[Sample],
    order: CicOrder,
    rate_factor: RateFactor,
) -> Result<Vec<Sample>, Error> {
    if order.value() == 0 {
        Err(Error::Cic("order must be at least 1".to_owned()))
    } else if rate_factor.value() == 0 {
        Err(Error::InvalidRateFactor {
            factor: rate_factor.value(),
        })
    } else {
        // Phase 1: cascade M integrators (each is a prefix sum)
        let integrated = (0..order.value()).fold(input.to_vec(), |data, _| prefix_sum(&data));

        // Phase 2: decimate by R
        let decimated: Vec<Sample> = integrated
            .iter()
            .step_by(rate_factor.value())
            .copied()
            .collect();

        // Phase 3: cascade M comb stages (each is a first difference)
        let combed = (0..order.value()).fold(decimated, |data, _| first_difference(&data));

        Ok(combed)
    }
}

/// CIC interpolation filter.
///
/// The reverse of decimation: comb -> upsample -> integrate.
///
/// # Errors
///
/// Returns [`Error::Cic`] if order is zero.
/// Returns [`Error::InvalidRateFactor`] if rate factor is zero.
pub fn cic_interpolate(
    input: &[Sample],
    order: CicOrder,
    rate_factor: RateFactor,
) -> Result<Vec<Sample>, Error> {
    if order.value() == 0 {
        Err(Error::Cic("order must be at least 1".to_owned()))
    } else if rate_factor.value() == 0 {
        Err(Error::InvalidRateFactor {
            factor: rate_factor.value(),
        })
    } else {
        // Phase 1: cascade M comb stages at input rate
        let combed = (0..order.value()).fold(input.to_vec(), |data, _| first_difference(&data));

        // Phase 2: upsample by R (insert R-1 zeros)
        let upsampled: Vec<Sample> = combed
            .iter()
            .flat_map(|s| {
                std::iter::once(*s)
                    .chain(std::iter::repeat_n(Sample::ZERO, rate_factor.value() - 1))
            })
            .collect();

        // Phase 3: cascade M integrators at output rate
        let integrated = (0..order.value()).fold(upsampled, |data, _| prefix_sum(&data));

        Ok(integrated)
    }
}

/// Running prefix sum (single integrator stage).
fn prefix_sum(input: &[Sample]) -> Vec<Sample> {
    input
        .iter()
        .scan(Sample::ZERO, |acc, s| {
            *acc = *acc + *s;
            Some(*acc)
        })
        .collect()
}

/// First difference (single comb stage with unit delay).
fn first_difference(input: &[Sample]) -> Vec<Sample> {
    input
        .first()
        .copied()
        .into_iter()
        .chain(input.windows(2).map(|w| w[1] - w[0]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_zero_is_error() {
        let result = cic_decimate(&[Sample::new(1)], CicOrder::new(0), RateFactor::new(2));
        assert!(result.is_err());
    }

    #[test]
    fn rate_factor_zero_is_error() {
        let result = cic_decimate(&[Sample::new(1)], CicOrder::new(1), RateFactor::new(0));
        assert!(result.is_err());
    }

    #[test]
    fn order_1_rate_1_is_identity() -> Result<(), Error> {
        let input: Vec<Sample> = (1..=4).map(Sample::new).collect();
        let output = cic_decimate(&input, CicOrder::new(1), RateFactor::new(1))?;
        // integrate [1,2,3,4] -> [1,3,6,10]
        // decimate by 1 -> [1,3,6,10]
        // comb (first diff) -> [1,2,3,4]
        assert_eq!(output, input);
        Ok(())
    }

    #[test]
    fn order_2_rate_2() -> Result<(), Error> {
        let input: Vec<Sample> = (1..=8).map(Sample::new).collect();
        let output = cic_decimate(&input, CicOrder::new(2), RateFactor::new(2))?;
        // Double integrate, decimate, double comb
        assert_eq!(output.len(), 4);
        Ok(())
    }

    #[test]
    fn constant_input_through_cic() -> Result<(), Error> {
        // Constant input through CIC should produce constant output (after settling)
        let input: Vec<Sample> = std::iter::repeat_n(Sample::new(1), 16).collect();
        let output = cic_decimate(&input, CicOrder::new(1), RateFactor::new(4))?;
        // After settling, each output should be 4 (sum of 4 ones per decimated sample)
        // Last output: difference of prefix sums separated by 4
        assert_eq!(output.len(), 4);
        // First output is 1 (prefix_sum[0] = 1, comb of first = 1)
        // Then differences: 4, 4, 4
        assert_eq!(output.last().map(|s| s.value()), Some(4));
        Ok(())
    }

    #[test]
    fn empty_input_produces_empty() -> Result<(), Error> {
        let output = cic_decimate(&[], CicOrder::new(1), RateFactor::new(2))?;
        assert!(output.is_empty());
        Ok(())
    }

    #[test]
    fn prefix_sum_correctness() {
        let input: Vec<Sample> = (1..=4).map(Sample::new).collect();
        let output = prefix_sum(&input);
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![1, 3, 6, 10]);
    }

    #[test]
    fn first_difference_correctness() {
        let input: Vec<Sample> = vec![1, 3, 6, 10].into_iter().map(Sample::new).collect();
        let output = first_difference(&input);
        let values: Vec<i32> = output.iter().map(|s| s.value()).collect();
        assert_eq!(values, vec![1, 2, 3, 4]);
    }
}
