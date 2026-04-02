# dsp-cat

Composable DSP signal processing pipeline in `RustHDL`, with
categorical pipeline assembly driven by `comp-cat-rs`.

## Overview

This crate provides:

- **Sample type**: `Sample(i32)` newtype with saturating fixed-point
  arithmetic and `SampleFormat` for tracking bit width and fractional
  bits through the pipeline.
- **DSP blocks**: FIR filter, CIC decimation filter, delay line, gain,
  decimator, interpolator, accumulator.
- **Golden models**: Pure reference implementations of every block,
  composable into arbitrary pipelines.
- **Categorical pipeline description**: The pipeline topology is a
  free category graph (`comp_cat_rs::collapse::free_category`).
  Each block is an edge; the `interpret()` universal property composes
  them into a single `DspBlockDescriptor`.
- **`RustHDL` modules**: `LogicBlock` implementations for every DSP
  block, plus a composed `DspPipeline` that chains blocks.
- **`Io`-wrapped simulation**: Behavioral simulation with all side
  effects deferred inside `comp_cat_rs::effect::io::Io::suspend`.

## Architecture

```text
Layer 1 (Pure)                    Layer 2 (HDL)
--------------------              --------------------
sample/                           hdl/
  element.rs  (Sample newtype)      delay.rs      (circular buffer)
  format.rs   (SampleFormat)        gain.rs       (1-cycle multiply)
                                    decimator.rs  (keep every Nth)
golden/                             interpolator.rs (zero insertion)
  fir.rs      (convolution)         accumulator.rs  (running sum)
  cic.rs      (integrate/decimate)
  delay.rs    (prepend zeros)     hdl/fir/
  gain.rs     (scale)               mac.rs        (multiply-accumulate)
  decimator.rs (downsample)         tap_chain.rs  (shift register)
  interpolator.rs (upsample)        fir_filter.rs (direct-form FIR)
  accumulator.rs (prefix sum)
  pipeline.rs (composed golden)   hdl/cic/
                                    integrator.rs (running sum stage)
graph/                              comb.rs       (first difference)
  pipeline_graph.rs (N+1 V, N E)   cic_filter.rs (M int + dec + M comb)
  fir_graph.rs (6V, 7E FSM)
  cic_graph.rs (7V, 8E FSM)      hdl/
                                    pipeline.rs   (composed chain)
interpret/
  signal.rs     (BoundarySignal) sim/
  descriptor.rs (DspBlockDescriptor) runner.rs   (Io-wrapped sim)
  morphism.rs   (GraphMorphism)

composition/
  cascade.rs    (sequential)
  parallel.rs   (tensor product)
```

**Layer 1** is pure: zero `mut`, combinators only, comp-cat-rs effects.
**Layer 2** quarantines `mut` inside `RustHDL`'s `Logic::update` methods
and `Io::suspend` closures at the simulation boundary.

The bridge between layers is the `interpret()` universal property of the
free category: it maps the abstract pipeline graph into concrete block
descriptors, which Layer 2 materializes into `RustHDL` modules.

## DSP Blocks

| Block | Golden model | HDL | Latency |
|---|---|---|---|
| FIR filter | `fir_convolve` | `FirFilterHdl` | N taps |
| CIC filter | `cic_decimate` / `cic_interpolate` | `CicFilterHdl` | 2M+1 cycles |
| Delay line | `delay_line` | `DelayLineHdl` | depth cycles |
| Gain | `apply_gain` | `GainHdl` | 1 cycle |
| Decimator | `decimate` | `DecimatorHdl` | 1 cycle |
| Interpolator | `interpolate` | `InterpolatorHdl` | 1 cycle |
| Accumulator | `accumulate` | `AccumulatorHdl` | 1 cycle |

## Usage

```rust
use dsp_cat::golden::fir::fir_convolve;
use dsp_cat::interpret::signal::FractionalBits;
use dsp_cat::sample::element::Sample;

// FIR convolution: impulse response = coefficients
let impulse = vec![Sample::new(1), Sample::ZERO, Sample::ZERO];
let coeffs = vec![Sample::new(3), Sample::new(2), Sample::new(1)];
let output = fir_convolve(&impulse, &coeffs, FractionalBits::new(0)).ok();
```

```rust
use dsp_cat::graph::pipeline_graph::{PipelineGraph, full_pipeline_path};
use dsp_cat::interpret::morphism::{BlockConfig, DspPipelineInterpretation};
use dsp_cat::interpret::descriptor::DspBlockDescriptor;
use dsp_cat::interpret::signal::{BitWidth, DelayDepth, FractionalBits, GainCoefficient};
use dsp_cat::sample::format::SampleFormat;
use comp_cat_rs::collapse::free_category::interpret;

// Compose a pipeline via the free category universal property
let graph = PipelineGraph::new(2);
let interp = DspPipelineInterpretation::new(
    vec![
        BlockConfig::Delay { depth: DelayDepth::new(4) },
        BlockConfig::Gain { coefficient: GainCoefficient::new(3) },
    ],
    SampleFormat::new(BitWidth::new(32), FractionalBits::new(15)),
);
let path = full_pipeline_path(&graph).ok();
let desc = path.map(|p| interpret::<PipelineGraph, _>(
    &interp,
    &p,
    |_| DspBlockDescriptor::identity(),
    DspBlockDescriptor::compose,
));
// desc.block_count() == 2
```

```rust
use dsp_cat::sim::runner::{SimConfig, simulate_pipeline};
use dsp_cat::interpret::descriptor::DspBlockDescriptor;
use dsp_cat::interpret::signal::{BlockIndex, GainCoefficient};
use dsp_cat::sample::element::Sample;

// Behavioral simulation with deferred execution
let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(2));
let config = SimConfig::new(vec![Sample::new(10)], desc);

// Nothing executes until .run()
let result = simulate_pipeline(config).run().ok();
```

## comp-cat-rs Integration

| comp-cat-rs concept | dsp-cat mapping |
|---|---|
| `Graph` | `PipelineGraph`: N+1 vertices (boundaries), N edges (blocks) |
| `Path` | Full pipeline path: N composed singleton edges |
| `GraphMorphism` | `DspPipelineInterpretation`: vertex to `BoundarySignal`, edge to `DspBlockDescriptor` |
| `interpret()` | Composes N descriptors into one pipeline descriptor |
| `Io<Error, _>` | Wraps simulation side effects; `.run()` at boundary only |
| Tensor product | `parallel()`: independent channel processing |

## Building

```sh
cargo build
cargo test
RUSTFLAGS="-D warnings" cargo clippy
cargo doc --no-deps --open
```

## Testing

118 tests across four levels:

- **Unit tests** (90): sample arithmetic, descriptor composition,
  graph connectivity, golden model properties, interpretation
  correctness, pipeline construction.
- **Integration tests** (11): golden model vs simulation agreement,
  pipeline composition via free category.
- **Doctests** (17): all public API examples.
- **Benchmarks**: FIR pipeline via `criterion`.

```sh
cargo test          # all unit + integration tests
cargo test --doc    # doctests
cargo bench         # criterion benchmarks
```

## License

Licensed under either of

- MIT license
- Apache License, Version 2.0

at your option.
