# dsp-cat

Composable DSP signal processing pipeline with `hdl-cat` Mealy
machines and categorical pipeline assembly driven by `comp-cat-rs`.

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
- **`hdl-cat` Sync machines**: Each DSP block is an IR graph built
  with `HdlGraphBuilder`, returned as a `RawDspBlock`.  Pipelines
  are composed via `compose_raw` (folded graph merging).
- **`Io`-wrapped simulation**: Behavioral simulation with all side
  effects deferred inside `comp_cat_rs::effect::io::Io::suspend`.

## Architecture

```text
Layer 1 (Pure)                    Layer 2 (HDL)
--------------------              --------------------
sample/                           hdl/
  element.rs  (Sample newtype)      common.rs     (DspIo, IR helpers)
  format.rs   (SampleFormat)        raw.rs        (RawDspBlock, compose_raw)
                                    accumulator.rs (running sum)
golden/                             gain.rs       (widening multiply + shift)
  fir.rs      (convolution)         delay.rs      (shift register chain)
  cic.rs      (integrate/decimate)  decimator.rs  (keep every Nth)
  delay.rs    (prepend zeros)       interpolator.rs (zero insertion FSM)
  gain.rs     (scale)               fir.rs        (unrolled MAC chain)
  decimator.rs (downsample)         cic.rs        (integrator + dec + comb)
  interpolator.rs (upsample)        pipeline.rs   (descriptor -> RawDspBlock)
  accumulator.rs (prefix sum)
  pipeline.rs (composed golden)   sim/
                                    runner.rs     (Io-wrapped sim)
graph/
  pipeline_graph.rs (N+1 V, N E)
  fir_graph.rs (6V, 7E FSM)
  cic_graph.rs (7V, 8E FSM)

interpret/
  signal.rs     (BoundarySignal)
  descriptor.rs (DspBlockDescriptor)
  morphism.rs   (GraphMorphism)

composition/
  cascade.rs    (sequential)
  parallel.rs   (tensor product)
```

**Layer 1** is pure: zero `mut`, combinators only, comp-cat-rs effects.
**Layer 2** builds `hdl-cat` IR graphs via the functional
`HdlGraphBuilder`.  All construction is pure; `mut` is confined to
`Io::suspend` closures at the simulation boundary.

The bridge between layers is the `interpret()` universal property of the
free category: it maps the abstract pipeline graph into concrete block
descriptors, which Layer 2 materializes into `hdl-cat` `Sync` machines
via `build_pipeline`.

## DSP Blocks

| Block | Golden model | HDL constructor | Latency |
|---|---|---|---|
| FIR filter | `fir_convolve` | `build_fir` | combinational |
| CIC filter | `cic_decimate` / `cic_interpolate` | `build_cic` | combinational |
| Delay line | `delay_line` | `build_delay` | depth cycles |
| Gain | `apply_gain` | `build_gain` | 1 cycle |
| Decimator | `decimate` | `build_decimator` | combinational |
| Interpolator | `interpolate` | `build_interpolator` | multi-cycle FSM |
| Accumulator | `accumulate` | `build_accumulator` | 1 cycle |

Every HDL constructor returns a `RawDspBlock` containing an IR graph,
wire layout, and initial state.  Pipelines are assembled by folding
blocks with `compose_raw`, which replicates `hdl-cat`'s
`compose_sync` graph-merge logic at the raw level.

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

```rust
use dsp_cat::hdl::pipeline::build_pipeline;
use dsp_cat::interpret::descriptor::DspBlockDescriptor;
use dsp_cat::interpret::signal::{BlockIndex, GainCoefficient, DelayDepth};

// Build an hdl-cat IR graph from a descriptor
let desc = DspBlockDescriptor::gain(BlockIndex::new(0), GainCoefficient::new(3))
    .compose(DspBlockDescriptor::delay(BlockIndex::new(1), DelayDepth::new(2)));
let raw_block = build_pipeline(&desc);
// raw_block contains the merged IR graph, wire layout, and initial state
// suitable for hdl-cat Testbench simulation or Verilog emission.
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

146 tests across four levels:

- **Unit tests** (118): sample arithmetic, descriptor composition,
  graph connectivity, golden model properties, interpretation
  correctness, pipeline construction, IR graph structure.
- **Integration tests** (7): golden model vs simulation agreement,
  pipeline composition via free category.
- **Pipeline composition tests** (4): free category cascade and
  interpretation properties.
- **Doctests** (17): all public API examples.

```sh
cargo test          # all unit + integration tests
cargo test --doc    # doctests
```

## License

Licensed under either of

- MIT license
- Apache License, Version 2.0

at your option.
