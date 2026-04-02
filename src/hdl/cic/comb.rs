//! Single comb stage HDL module.
//!
//! First difference: `y[n] = x[n] - x[n-1]`.  Used as a building
//! block for CIC filters.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, i32_to_bits, SAMPLE_WIDTH};

/// Single comb (first difference) stage.
///
/// Latency: 1 clock cycle.
#[derive(Clone, Debug, Default, LogicBlock)]
pub struct CombStage {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Differenced output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    prev: DFF<Bits<SAMPLE_WIDTH>>,
    valid_reg: DFF<Bit>,
}

impl Logic for CombStage {
    fn update(&mut self) {
        self.prev.clock.next = self.clock.val();
        self.valid_reg.clock.next = self.clock.val();

        let current = bits_to_i32(self.data_in.val());
        let previous = bits_to_i32(self.prev.q.val());

        if self.valid_in.val() {
            self.prev.d.next = self.data_in.val();
        } else {
            self.prev.d.next = self.prev.q.val();
        }
        self.valid_reg.d.next = self.valid_in.val();

        self.data_out.next = i32_to_bits(current.saturating_sub(previous));
        self.valid_out.next = self.valid_reg.q.val();
    }
}
