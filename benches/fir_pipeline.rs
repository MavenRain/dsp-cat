//! FIR pipeline benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dsp_cat::golden::fir::fir_convolve;
use dsp_cat::interpret::signal::FractionalBits;
use dsp_cat::sample::element::Sample;

fn fir_64_taps_1024_samples(c: &mut Criterion) {
    let input: Vec<Sample> = (0..1024).map(Sample::new).collect();
    let coeffs: Vec<Sample> = (0..64).map(|i| Sample::new(i % 7)).collect();
    let frac = FractionalBits::new(0);

    c.bench_function("fir_64_taps_1024_samples", |b| {
        b.iter(|| fir_convolve(black_box(&input), black_box(&coeffs), black_box(frac)));
    });
}

fn fir_16_taps_256_samples(c: &mut Criterion) {
    let input: Vec<Sample> = (0..256).map(Sample::new).collect();
    let coeffs: Vec<Sample> = (0..16).map(|i| Sample::new(i + 1)).collect();
    let frac = FractionalBits::new(0);

    c.bench_function("fir_16_taps_256_samples", |b| {
        b.iter(|| fir_convolve(black_box(&input), black_box(&coeffs), black_box(frac)));
    });
}

criterion_group!(benches, fir_64_taps_1024_samples, fir_16_taps_256_samples);
criterion_main!(benches);
