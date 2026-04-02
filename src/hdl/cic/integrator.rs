//! Single integrator stage HDL module.
//!
//! Running sum: `y[n] = y[n-1] + x[n]`.  Used as a building block
//! for CIC filters.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, i32_to_bits, SAMPLE_WIDTH};

/// Single integrator (running sum) stage.
///
/// Latency: 1 clock cycle.
#[derive(Clone, Debug, Default, LogicBlock)]
pub struct IntegratorStage {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Integrated output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    accum: DFF<Bits<SAMPLE_WIDTH>>,
    valid_reg: DFF<Bit>,
}

impl Logic for IntegratorStage {
    fn update(&mut self) {
        self.accum.clock.next = self.clock.val();
        self.valid_reg.clock.next = self.clock.val();

        let current = bits_to_i32(self.accum.q.val());
        let input = bits_to_i32(self.data_in.val());

        if self.valid_in.val() {
            self.accum.d.next = i32_to_bits(current.saturating_add(input));
        } else {
            self.accum.d.next = self.accum.q.val();
        }
        self.valid_reg.d.next = self.valid_in.val();

        self.data_out.next = i32_to_bits(current.saturating_add(input));
        self.valid_out.next = self.valid_reg.q.val();
    }
}
