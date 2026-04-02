//! Pipeline composition tests via the free category mechanism.

use comp_cat_rs::collapse::free_category::interpret;
use dsp_cat::composition::cascade::cascade;
use dsp_cat::error::Error;
use dsp_cat::graph::pipeline_graph::{full_pipeline_path, sub_pipeline_path, PipelineGraph};
use dsp_cat::interpret::descriptor::DspBlockDescriptor;
use dsp_cat::interpret::morphism::{BlockConfig, DspPipelineInterpretation};
use dsp_cat::interpret::signal::{
    BitWidth, CicOrder, DelayDepth, FractionalBits, GainCoefficient, RateFactor,
};
use dsp_cat::sample::format::SampleFormat;

fn default_format() -> SampleFormat {
    SampleFormat::new(BitWidth::new(32), FractionalBits::new(15))
}

#[test]
fn cascade_produces_correct_block_count() -> Result<(), Error> {
    let desc = cascade(&[
        BlockConfig::Delay {
            depth: DelayDepth::new(4),
        },
        BlockConfig::Gain {
            coefficient: GainCoefficient::new(2),
        },
        BlockConfig::Accumulator,
    ])?;
    assert_eq!(desc.block_count(), 3);
    Ok(())
}

#[test]
fn sub_pipeline_interpret_matches_subrange() -> Result<(), Error> {
    let configs = vec![
        BlockConfig::Delay {
            depth: DelayDepth::new(1),
        },
        BlockConfig::Gain {
            coefficient: GainCoefficient::new(2),
        },
        BlockConfig::Accumulator,
        BlockConfig::Delay {
            depth: DelayDepth::new(3),
        },
    ];
    let graph = PipelineGraph::new(configs.len());
    let interp = DspPipelineInterpretation::new(configs, default_format());

    let sub = sub_pipeline_path(&graph, 1, 3)?;
    let desc = interpret::<PipelineGraph, _>(
        &interp,
        &sub,
        |_| DspBlockDescriptor::identity(),
        DspBlockDescriptor::compose,
    );
    assert_eq!(desc.block_count(), 2);
    Ok(())
}

#[test]
fn full_path_interpret_equals_cascade() -> Result<(), Error> {
    let configs = vec![
        BlockConfig::Delay {
            depth: DelayDepth::new(4),
        },
        BlockConfig::Cic {
            order: CicOrder::new(2),
            rate_factor: RateFactor::new(4),
        },
    ];

    let from_cascade = cascade(&configs)?;

    let graph = PipelineGraph::new(configs.len());
    let interp = DspPipelineInterpretation::new(configs, default_format());
    let path = full_pipeline_path(&graph)?;
    let from_interpret = interpret::<PipelineGraph, _>(
        &interp,
        &path,
        |_| DspBlockDescriptor::identity(),
        DspBlockDescriptor::compose,
    );

    assert_eq!(from_cascade.block_count(), from_interpret.block_count());
    Ok(())
}

#[test]
fn empty_cascade_is_identity() -> Result<(), Error> {
    let desc = cascade(&[])?;
    assert_eq!(desc.block_count(), 0);
    Ok(())
}
