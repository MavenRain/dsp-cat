//! Gain (scale) HDL module.
//!
//! Multiplies each input sample by a fixed coefficient.
//! 1-cycle registered output.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, clamp_to_i32, i32_to_bits, SAMPLE_WIDTH};

/// Gain block with fixed coefficient.
///
/// Latency: 1 clock cycle.
#[derive(Clone, Debug)]
pub struct GainHdl {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Scaled sample output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    out_reg: DFF<Bits<SAMPLE_WIDTH>>,
    valid_reg: DFF<Bit>,
    coefficient: i32,
    shift: u32,
}

impl GainHdl {
    /// Create a gain block with the given coefficient and right-shift.
    #[must_use]
    pub fn new(coefficient: i32, shift: u32) -> Self {
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            data_out: Signal::default(),
            valid_out: Signal::default(),
            out_reg: DFF::default(),
            valid_reg: DFF::default(),
            coefficient,
            shift,
        }
    }
}

impl Default for GainHdl {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

impl Logic for GainHdl {
    fn update(&mut self) {
        self.out_reg.clock.next = self.clock.val();
        self.valid_reg.clock.next = self.clock.val();

        let input = i64::from(bits_to_i32(self.data_in.val()));
        let product = input * i64::from(self.coefficient);
        let shifted = product >> self.shift;
        self.out_reg.d.next = i32_to_bits(clamp_to_i32(shifted));
        self.valid_reg.d.next = self.valid_in.val();

        self.data_out.next = self.out_reg.q.val();
        self.valid_out.next = self.valid_reg.q.val();
    }

    fn connect(&mut self) {
        self.data_out.connect();
        self.valid_out.connect();
    }

    fn hdl(&self) -> Verilog {
        Verilog::Empty
    }
}

impl Block for GainHdl {
    fn connect_all(&mut self) {
        self.out_reg.connect_all();
        self.valid_reg.connect_all();
        self.connect();
    }

    fn update_all(&mut self) {
        self.out_reg.update_all();
        self.valid_reg.update_all();
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
