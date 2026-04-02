//! Multiply-accumulate unit for FIR filter.
//!
//! Computes `accumulator += sample * coefficient` with widening
//! multiply to 64 bits, then truncates back to 32.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, clamp_to_i32, i32_to_bits, SAMPLE_WIDTH};

/// Single multiply-accumulate unit.
///
/// Latency: 1 clock cycle.
#[derive(Clone, Debug)]
pub struct MacUnit {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Input sample.
    pub sample: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Filter coefficient.
    pub coefficient: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Accumulated input from previous tap (or 0 for first tap).
    pub accum_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Accumulated output for next tap.
    pub accum_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    out_reg: DFF<Bits<SAMPLE_WIDTH>>,
    valid_reg: DFF<Bit>,
    shift: u32,
}

impl MacUnit {
    /// Create a MAC unit with the given fractional-bit shift.
    #[must_use]
    pub fn new(shift: u32) -> Self {
        Self {
            clock: Signal::default(),
            sample: Signal::default(),
            coefficient: Signal::default(),
            accum_in: Signal::default(),
            valid_in: Signal::default(),
            accum_out: Signal::default(),
            valid_out: Signal::default(),
            out_reg: DFF::default(),
            valid_reg: DFF::default(),
            shift,
        }
    }
}

impl Default for MacUnit {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Logic for MacUnit {
    fn update(&mut self) {
        self.out_reg.clock.next = self.clock.val();
        self.valid_reg.clock.next = self.clock.val();

        let s = i64::from(bits_to_i32(self.sample.val()));
        let c = i64::from(bits_to_i32(self.coefficient.val()));
        let product = (s * c) >> self.shift;
        let acc = i64::from(bits_to_i32(self.accum_in.val()));
        let sum = acc.saturating_add(product);
        self.out_reg.d.next = i32_to_bits(clamp_to_i32(sum));
        self.valid_reg.d.next = self.valid_in.val();

        self.accum_out.next = self.out_reg.q.val();
        self.valid_out.next = self.valid_reg.q.val();
    }

    fn connect(&mut self) {
        self.accum_out.connect();
        self.valid_out.connect();
    }

    fn hdl(&self) -> Verilog {
        Verilog::Empty
    }
}

impl Block for MacUnit {
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
        self.accum_out.changed() || self.valid_out.changed()
    }

    fn accept(&self, name: &str, probe: &mut dyn Probe) {
        probe.visit_start_scope(name, self);
        probe.visit_atom("accum_out", &self.accum_out);
        probe.visit_atom("valid_out", &self.valid_out);
        probe.visit_end_scope(name, self);
    }
}
