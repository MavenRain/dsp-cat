//! Decimator HDL module.
//!
//! Passes every Nth sample, discarding the rest.

use rust_hdl::prelude::*;

use crate::hdl::common::SAMPLE_WIDTH;

/// Decimator block.
///
/// Latency: 1 clock cycle (for the first kept sample).
#[derive(Clone, Debug)]
pub struct DecimatorHdl {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Decimated sample output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe (asserted every Nth cycle).
    pub valid_out: Signal<Out, Bit>,
    counter: usize,
    factor: usize,
}

impl DecimatorHdl {
    /// Create a decimator with the given factor.
    #[must_use]
    pub fn new(factor: usize) -> Self {
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            data_out: Signal::default(),
            valid_out: Signal::default(),
            counter: 0,
            factor: factor.max(1),
        }
    }
}

impl Default for DecimatorHdl {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Logic for DecimatorHdl {
    fn update(&mut self) {
        self.data_out.next = self.data_in.val();

        if self.valid_in.val() {
            self.valid_out.next = self.counter == 0;
            self.counter = (self.counter + 1) % self.factor;
        } else {
            self.valid_out.next = false;
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

impl Block for DecimatorHdl {
    fn connect_all(&mut self) {
        self.connect();
    }

    fn update_all(&mut self) {
        self.update();
    }

    fn has_changed(&self) -> bool {
        self.data_out.changed() || self.valid_out.changed()
    }

    fn accept(&self, name: &str, probe: &mut dyn Probe) {
        probe.visit_start_scope(name, self);
        probe.visit_atom("data_out", &self.data_out);
        probe.visit_atom("valid_out", &self.valid_out);
        probe.visit_end_scope(name, self);
    }
}
