//! Composed DSP pipeline HDL module.
//!
//! Chains multiple DSP blocks by wiring the output of block k
//! to the input of block k+1, following the same pattern as
//! `NttPipeline` in goldilocks-ntt-hdl.

use rust_hdl::prelude::*;

use crate::hdl::accumulator::AccumulatorHdl;
use crate::hdl::cic::cic_filter::CicFilterHdl;
use crate::hdl::common::{i32_to_bits, SAMPLE_WIDTH};
use crate::hdl::decimator::DecimatorHdl;
use crate::hdl::delay::DelayLineHdl;
use crate::hdl::fir::fir_filter::FirFilterHdl;
use crate::hdl::gain::GainHdl;
use crate::hdl::interpolator::InterpolatorHdl;
use crate::interpret::descriptor::DspBlockDescriptor;

/// A single DSP block, type-erased for pipeline composition.
#[derive(Clone, Debug)]
enum DspBlock {
    Delay(DelayLineHdl),
    Gain(GainHdl),
    Decimator(DecimatorHdl),
    Interpolator(InterpolatorHdl),
    Accumulator(AccumulatorHdl),
    Fir(FirFilterHdl),
    Cic(CicFilterHdl),
}

/// Composed DSP pipeline.
///
/// Built from a [`DspBlockDescriptor`] via [`DspPipeline::from_descriptor`].
#[derive(Clone, Debug)]
pub struct DspPipeline {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Pipeline output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    blocks: Vec<DspBlock>,
}

impl DspPipeline {
    /// Build a pipeline from a block descriptor.
    ///
    /// Flattens composed descriptors into a linear chain.
    #[must_use]
    pub fn from_descriptor(descriptor: &DspBlockDescriptor) -> Self {
        let blocks = Self::flatten_descriptor(descriptor);
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            data_out: Signal::default(),
            valid_out: Signal::default(),
            blocks,
        }
    }

    fn flatten_descriptor(desc: &DspBlockDescriptor) -> Vec<DspBlock> {
        match desc {
            DspBlockDescriptor::Identity => vec![],
            DspBlockDescriptor::Delay { depth, .. } => {
                vec![DspBlock::Delay(DelayLineHdl::new(depth.value()))]
            }
            DspBlockDescriptor::Gain {
                coefficient, shift, ..
            } => vec![DspBlock::Gain(GainHdl::new(coefficient.value(), *shift))],
            DspBlockDescriptor::Decimator { factor, .. } => {
                vec![DspBlock::Decimator(DecimatorHdl::new(factor.value()))]
            }
            DspBlockDescriptor::Interpolator { factor, .. } => {
                vec![DspBlock::Interpolator(InterpolatorHdl::new(factor.value()))]
            }
            DspBlockDescriptor::Accumulator { .. } => {
                vec![DspBlock::Accumulator(AccumulatorHdl::default())]
            }
            DspBlockDescriptor::Fir {
                coefficients,
                frac_bits,
                ..
            } => {
                #[allow(clippy::cast_possible_truncation)]
                let shift = frac_bits.value() as u32;
                vec![DspBlock::Fir(FirFilterHdl::new(coefficients, shift))]
            }
            DspBlockDescriptor::Cic {
                order, rate_factor, ..
            } => vec![DspBlock::Cic(CicFilterHdl::new(
                order.value(),
                rate_factor.value(),
            ))],
            DspBlockDescriptor::Composed(blocks) => {
                blocks.iter().flat_map(Self::flatten_descriptor).collect()
            }
        }
    }
}

impl Default for DspPipeline {
    fn default() -> Self {
        Self::from_descriptor(&DspBlockDescriptor::Identity)
    }
}

/// Helper: get `data_out` and `valid_out` from a block.
fn block_outputs(block: &DspBlock) -> (Bits<SAMPLE_WIDTH>, bool) {
    match block {
        DspBlock::Delay(b) => (b.data_out.val(), b.valid_out.val()),
        DspBlock::Gain(b) => (b.data_out.val(), b.valid_out.val()),
        DspBlock::Decimator(b) => (b.data_out.val(), b.valid_out.val()),
        DspBlock::Interpolator(b) => (b.data_out.val(), b.valid_out.val()),
        DspBlock::Accumulator(b) => (b.data_out.val(), b.valid_out.val()),
        DspBlock::Fir(b) => (b.data_out.val(), b.valid_out.val()),
        DspBlock::Cic(b) => (b.data_out.val(), b.valid_out.val()),
    }
}

/// Helper: set clock, `data_in`, `valid_in` on a block.
fn set_block_inputs(
    block: &mut DspBlock,
    clock: Clock,
    data: Bits<SAMPLE_WIDTH>,
    valid: bool,
) {
    match block {
        DspBlock::Delay(b) => {
            b.clock.next = clock;
            b.data_in.next = data;
            b.valid_in.next = valid;
        }
        DspBlock::Gain(b) => {
            b.clock.next = clock;
            b.data_in.next = data;
            b.valid_in.next = valid;
        }
        DspBlock::Decimator(b) => {
            b.clock.next = clock;
            b.data_in.next = data;
            b.valid_in.next = valid;
        }
        DspBlock::Interpolator(b) => {
            b.clock.next = clock;
            b.data_in.next = data;
            b.valid_in.next = valid;
        }
        DspBlock::Accumulator(b) => {
            b.clock.next = clock;
            b.data_in.next = data;
            b.valid_in.next = valid;
        }
        DspBlock::Fir(b) => {
            b.clock.next = clock;
            b.data_in.next = data;
            b.valid_in.next = valid;
        }
        DspBlock::Cic(b) => {
            b.clock.next = clock;
            b.data_in.next = data;
            b.valid_in.next = valid;
        }
    }
}

/// Helper: call `connect_all` on a block variant.
fn connect_block(block: &mut DspBlock) {
    match block {
        DspBlock::Delay(b) => b.connect_all(),
        DspBlock::Gain(b) => b.connect_all(),
        DspBlock::Decimator(b) => b.connect_all(),
        DspBlock::Interpolator(b) => b.connect_all(),
        DspBlock::Accumulator(b) => b.connect_all(),
        DspBlock::Fir(b) => b.connect_all(),
        DspBlock::Cic(b) => b.connect_all(),
    }
}

/// Helper: call `update_all` on a block variant.
fn update_block(block: &mut DspBlock) {
    match block {
        DspBlock::Delay(b) => b.update_all(),
        DspBlock::Gain(b) => b.update_all(),
        DspBlock::Decimator(b) => b.update_all(),
        DspBlock::Interpolator(b) => b.update_all(),
        DspBlock::Accumulator(b) => b.update_all(),
        DspBlock::Fir(b) => b.update_all(),
        DspBlock::Cic(b) => b.update_all(),
    }
}

/// Helper: check if a block's outputs have changed.
fn block_has_changed(block: &DspBlock) -> bool {
    match block {
        DspBlock::Delay(b) => b.has_changed(),
        DspBlock::Gain(b) => b.has_changed(),
        DspBlock::Decimator(b) => b.has_changed(),
        DspBlock::Interpolator(b) => b.has_changed(),
        DspBlock::Accumulator(b) => b.has_changed(),
        DspBlock::Fir(b) => b.has_changed(),
        DspBlock::Cic(b) => b.has_changed(),
    }
}

/// Helper: accept a probe visitor into a block variant.
fn accept_block(block: &DspBlock, name: &str, probe: &mut dyn Probe) {
    match block {
        DspBlock::Delay(b) => b.accept(name, probe),
        DspBlock::Gain(b) => b.accept(name, probe),
        DspBlock::Decimator(b) => b.accept(name, probe),
        DspBlock::Interpolator(b) => b.accept(name, probe),
        DspBlock::Accumulator(b) => b.accept(name, probe),
        DspBlock::Fir(b) => b.accept(name, probe),
        DspBlock::Cic(b) => b.accept(name, probe),
    }
}

impl Logic for DspPipeline {
    fn update(&mut self) {
        let n = self.blocks.len();

        if n == 0 {
            self.data_out.next = self.data_in.val();
            self.valid_out.next = self.valid_in.val();
        } else {
            // Collect intermediate outputs (borrow checker pattern from NttPipeline)
            let intermediates: Vec<(Bits<SAMPLE_WIDTH>, bool)> =
                self.blocks.iter().map(block_outputs).collect();

            // Feed first block from pipeline input
            if let Some(first) = self.blocks.first_mut() {
                set_block_inputs(
                    first,
                    self.clock.val(),
                    self.data_in.val(),
                    self.valid_in.val(),
                );
            }

            // Chain: output of block k-1 feeds input of block k
            (1..n).for_each(|k| {
                if let (Some((data, valid)), Some(block)) =
                    (intermediates.get(k - 1), self.blocks.get_mut(k))
                {
                    set_block_inputs(block, self.clock.val(), *data, *valid);
                }
            });

            // Drive pipeline output from last block
            if let Some((data, valid)) = intermediates.last() {
                self.data_out.next = *data;
                self.valid_out.next = *valid;
            } else {
                self.data_out.next = i32_to_bits(0);
                self.valid_out.next = false;
            }
        }
    }

    fn connect(&mut self) {
        self.data_out.connect();
        self.valid_out.connect();
    }

    fn hdl(&self) -> Verilog {
        Verilog::Empty
    }
}

impl Block for DspPipeline {
    fn connect_all(&mut self) {
        self.blocks.iter_mut().for_each(connect_block);
        self.connect();
    }

    fn update_all(&mut self) {
        self.blocks.iter_mut().for_each(update_block);
        self.update();
    }

    fn has_changed(&self) -> bool {
        self.data_out.changed()
            || self.valid_out.changed()
            || self.blocks.iter().any(block_has_changed)
    }

    fn accept(&self, name: &str, probe: &mut dyn Probe) {
        probe.visit_start_scope(name, self);
        self.blocks.iter().enumerate().for_each(|(i, block)| {
            accept_block(block, &format!("block_{i}"), probe);
        });
        probe.visit_atom("data_out", &self.data_out);
        probe.visit_atom("valid_out", &self.valid_out);
        probe.visit_end_scope(name, self);
    }
}
