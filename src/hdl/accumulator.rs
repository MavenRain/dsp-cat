//! Accumulator (running sum) HDL module.
//!
//! Single integrator stage: `y[n] = y[n-1] + x[n]`.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, i32_to_bits, SAMPLE_WIDTH};

/// Running-sum accumulator.
///
/// Latency: 1 clock cycle.
#[derive(Clone, Debug, Default, LogicBlock)]
pub struct AccumulatorHdl {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Accumulated output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    accum_reg: DFF<Bits<SAMPLE_WIDTH>>,
    valid_reg: DFF<Bit>,
}

impl Logic for AccumulatorHdl {
    fn update(&mut self) {
        self.accum_reg.clock.next = self.clock.val();
        self.valid_reg.clock.next = self.clock.val();

        let current = bits_to_i32(self.accum_reg.q.val());
        let input = bits_to_i32(self.data_in.val());
        let sum = current.saturating_add(input);

        if self.valid_in.val() {
            self.accum_reg.d.next = i32_to_bits(sum);
        } else {
            self.accum_reg.d.next = self.accum_reg.q.val();
        }
        self.valid_reg.d.next = self.valid_in.val();

        self.data_out.next = i32_to_bits(sum);
        self.valid_out.next = self.valid_reg.q.val();
    }
}
