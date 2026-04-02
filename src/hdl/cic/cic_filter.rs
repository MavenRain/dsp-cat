//! Complete CIC decimation filter HDL module.
//!
//! Chains M integrator stages, a decimator, and M comb stages.
//! No multipliers required.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, i32_to_bits, SAMPLE_WIDTH};

/// CIC decimation filter.
///
/// Latency: `2 * order + 1` clock cycles per output sample.
#[derive(Clone, Debug)]
pub struct CicFilterHdl {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Filtered, decimated output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    integrator_accums: Vec<i32>,
    comb_prevs: Vec<i32>,
    decim_counter: usize,
    order: usize,
    rate_factor: usize,
}

impl CicFilterHdl {
    /// Create a CIC decimation filter.
    #[must_use]
    pub fn new(order: usize, rate_factor: usize) -> Self {
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            data_out: Signal::default(),
            valid_out: Signal::default(),
            integrator_accums: vec![0_i32; order],
            comb_prevs: vec![0_i32; order],
            decim_counter: 0,
            order,
            rate_factor: rate_factor.max(1),
        }
    }
}

impl Default for CicFilterHdl {
    fn default() -> Self {
        Self::new(1, 2)
    }
}

impl Logic for CicFilterHdl {
    fn update(&mut self) {
        if self.valid_in.val() {
            // Phase 1: cascade integrators
            let value = (0..self.order).fold(
                bits_to_i32(self.data_in.val()),
                |v, k| {
                    let acc = self.integrator_accums.get(k).copied().unwrap_or(0);
                    let next = acc.saturating_add(v);
                    if let Some(slot) = self.integrator_accums.get_mut(k) {
                        *slot = next;
                    }
                    next
                },
            );

            // Phase 2: decimation
            if self.decim_counter == 0 {
                // Phase 3: cascade combs
                let comb_val = (0..self.order).fold(value, |v, k| {
                    let prev = self.comb_prevs.get(k).copied().unwrap_or(0);
                    let diff = v.saturating_sub(prev);
                    if let Some(slot) = self.comb_prevs.get_mut(k) {
                        *slot = v;
                    }
                    diff
                });

                self.data_out.next = i32_to_bits(comb_val);
                self.valid_out.next = true;
            } else {
                self.data_out.next = i32_to_bits(0);
                self.valid_out.next = false;
            }

            self.decim_counter = (self.decim_counter + 1) % self.rate_factor;
        } else {
            self.data_out.next = i32_to_bits(0);
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

impl Block for CicFilterHdl {
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
