//! Interpolator HDL module.
//!
//! Inserts `factor - 1` zero-valued samples between each input sample.

use rust_hdl::prelude::*;

use crate::hdl::common::{i32_to_bits, SAMPLE_WIDTH};

/// Interpolator block.
///
/// When a valid input arrives, outputs the sample followed by
/// `factor - 1` zeros on successive cycles.
#[derive(Clone, Debug)]
pub struct InterpolatorHdl {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Interpolated sample output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    /// Back-pressure: ready to accept new input.
    pub ready: Signal<Out, Bit>,
    counter: usize,
    factor: usize,
    held_sample: Bits<SAMPLE_WIDTH>,
    active: bool,
}

impl InterpolatorHdl {
    /// Create an interpolator with the given factor.
    #[must_use]
    pub fn new(factor: usize) -> Self {
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            data_out: Signal::default(),
            valid_out: Signal::default(),
            ready: Signal::default(),
            counter: 0,
            factor: factor.max(1),
            held_sample: Bits::default(),
            active: false,
        }
    }
}

impl Default for InterpolatorHdl {
    fn default() -> Self {
        Self::new(2)
    }
}

impl Logic for InterpolatorHdl {
    fn update(&mut self) {
        if self.active {
            if self.counter == 0 {
                self.data_out.next = self.held_sample;
            } else {
                self.data_out.next = i32_to_bits(0);
            }
            self.valid_out.next = true;
            self.counter += 1;
            if self.counter >= self.factor {
                self.counter = 0;
                self.active = false;
            }
            self.ready.next = false;
        } else if self.valid_in.val() {
            self.held_sample = self.data_in.val();
            self.data_out.next = self.data_in.val();
            self.valid_out.next = true;
            self.counter = 1;
            if self.factor > 1 {
                self.active = true;
                self.ready.next = false;
            } else {
                self.ready.next = true;
            }
        } else {
            self.data_out.next = i32_to_bits(0);
            self.valid_out.next = false;
            self.ready.next = true;
        }
    }

    fn connect(&mut self) {
        self.data_out.connect();
        self.valid_out.connect();
        self.ready.connect();
    }

    fn hdl(&self) -> Verilog {
        Verilog::Empty
    }
}

impl Block for InterpolatorHdl {
    fn connect_all(&mut self) {
        self.connect();
    }

    fn update_all(&mut self) {
        self.update();
    }

    fn has_changed(&self) -> bool {
        self.data_out.changed() || self.valid_out.changed() || self.ready.changed()
    }

    fn accept(&self, name: &str, probe: &mut dyn Probe) {
        probe.visit_start_scope(name, self);
        probe.visit_atom("data_out", &self.data_out);
        probe.visit_atom("valid_out", &self.valid_out);
        probe.visit_atom("ready", &self.ready);
        probe.visit_end_scope(name, self);
    }
}
