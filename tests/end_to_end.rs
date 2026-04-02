//! End-to-end tests: golden model vs behavioral simulation.

use dsp_cat::error::Error;
use dsp_cat::golden::pipeline::pipeline_golden;
use dsp_cat::interpret::descriptor::DspBlockDescriptor;
use dsp_cat::interpret::signal::{
    BlockIndex, CicOrder, DelayDepth, FractionalBits, GainCoefficient, RateFactor,
};
use dsp_cat::sample::element::Sample;
use dsp_cat::sim::runner::{simulate_pipeline, SimConfig};

#[test]
fn passthrough_preserves_data() -> Result<(), Error> {
    let input: Vec<Sample> = (1..=8).map(Sample::new).collect();
    let config = SimConfig::new(input.clone(), DspBlockDescriptor::Identity);
    let result = simulate_pipeline(config).run()?;
    assert_eq!(result, input);
    Ok(())
}

#[test]
fn fir_impulse_response() -> Result<(), Error> {
    let impulse = vec![Sample::new(1), Sample::ZERO, Sample::ZERO, Sample::ZERO];
    let coeffs = vec![Sample::new(3), Sample::new(2), Sample::new(1)];
    let desc = DspBlockDescriptor::fir(BlockIndex::new(0), coeffs, FractionalBits::new(0));
    let result = simulate_pipeline(SimConfig::new(impulse, desc)).run()?;
    let values: Vec<i32> = result.iter().map(|s| s.value()).collect();
    assert_eq!(values, vec![3, 2, 1, 0]);
    Ok(())
}

#[test]
fn cic_order_1_rate_1_is_identity() -> Result<(), Error> {
    let input: Vec<Sample> = (1..=4).map(Sample::new).collect();
    let desc = DspBlockDescriptor::cic(BlockIndex::new(0), CicOrder::new(1), RateFactor::new(1));
    let result = simulate_pipeline(SimConfig::new(input.clone(), desc)).run()?;
    assert_eq!(result, input);
    Ok(())
}

#[test]
fn delay_then_gain() -> Result<(), Error> {
    let input = vec![Sample::new(5), Sample::new(10)];
    let desc = DspBlockDescriptor::delay(BlockIndex::new(0), DelayDepth::new(2))
        .compose(DspBlockDescriptor::gain(
            BlockIndex::new(1),
            GainCoefficient::new(3),
        ));
    let result = simulate_pipeline(SimConfig::new(input, desc)).run()?;
    let values: Vec<i32> = result.iter().map(|s| s.value()).collect();
    // Delay: [0, 0, 5, 10], then gain by 3: [0, 0, 15, 30]
    assert_eq!(values, vec![0, 0, 15, 30]);
    Ok(())
}

#[test]
fn decimation_halves_length() -> Result<(), Error> {
    let input: Vec<Sample> = (0..8).map(Sample::new).collect();
    let desc = DspBlockDescriptor::decimator(BlockIndex::new(0), RateFactor::new(2));
    let result = simulate_pipeline(SimConfig::new(input, desc)).run()?;
    assert_eq!(result.len(), 4);
    Ok(())
}

#[test]
fn interpolation_doubles_length() -> Result<(), Error> {
    let input: Vec<Sample> = (1..=4).map(Sample::new).collect();
    let desc = DspBlockDescriptor::interpolator(BlockIndex::new(0), RateFactor::new(2));
    let result = simulate_pipeline(SimConfig::new(input, desc)).run()?;
    assert_eq!(result.len(), 8);
    Ok(())
}

#[test]
fn golden_matches_sim_for_composed_pipeline() -> Result<(), Error> {
    let input: Vec<Sample> = (1..=16).map(Sample::new).collect();
    let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(2))
        .compose(DspBlockDescriptor::accumulator(BlockIndex::new(1)));
    let golden = pipeline_golden(&input, &desc)?;
    let sim = simulate_pipeline(SimConfig::new(input, desc)).run()?;
    assert_eq!(golden, sim);
    Ok(())
}
