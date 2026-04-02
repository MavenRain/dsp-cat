//! Complete FIR filter HDL module.
//!
//! Implements a direct-form FIR filter by iterating through the
//! tap chain and accumulating products.  Behavioral simulation
//! model (not synthesizable as-is, but cycle-accurate).

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, clamp_to_i32, i32_to_bits, SAMPLE_WIDTH};
use crate::sample::element::Sample;

/// FIR filter with configurable coefficients.
///
/// Latency: `tap_count` clock cycles per output sample.
/// Throughput: 1 sample per `tap_count` clocks (non-pipelined).
#[derive(Clone, Debug)]
pub struct FirFilterHdl {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Filtered sample output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    coefficients: Vec<i32>,
    history: Vec<i32>,
    write_ptr: usize,
    tap_count: usize,
    shift: u32,
}

impl FirFilterHdl {
    /// Create a FIR filter from the given coefficients and fractional shift.
    #[must_use]
    pub fn new(coefficients: &[Sample], shift: u32) -> Self {
        let coeff_values: Vec<i32> = coefficients.iter().map(|s| s.value()).collect();
        let tap_count = coeff_values.len();
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            data_out: Signal::default(),
            valid_out: Signal::default(),
            coefficients: coeff_values,
            history: vec![0_i32; tap_count],
            write_ptr: 0,
            tap_count,
            shift,
        }
    }
}

impl Default for FirFilterHdl {
    fn default() -> Self {
        Self::new(&[Sample::new(1)], 0)
    }
}

impl Logic for FirFilterHdl {
    fn update(&mut self) {
        if self.valid_in.val() {
            // Write new sample into circular buffer
            let input = bits_to_i32(self.data_in.val());
            if let Some(slot) = self.history.get_mut(self.write_ptr) {
                *slot = input;
            }

            // Compute convolution: sum(h[k] * x[n-k])
            let max_tap = self.tap_count.max(1);
            let (acc, _) = (0..self.tap_count).fold(
                (0_i64, self.write_ptr),
                |(acc, read_ptr), k| {
                    let sample = self.history.get(read_ptr).copied().unwrap_or(0);
                    let coeff = self.coefficients.get(k).copied().unwrap_or(0);
                    let next_acc =
                        acc.saturating_add(i64::from(sample) * i64::from(coeff));
                    let next_ptr = if read_ptr == 0 {
                        max_tap.saturating_sub(1)
                    } else {
                        read_ptr - 1
                    };
                    (next_acc, next_ptr)
                },
            );

            let shifted = acc >> self.shift;
            self.data_out.next = i32_to_bits(clamp_to_i32(shifted));
            self.valid_out.next = true;

            self.write_ptr = (self.write_ptr + 1) % max_tap;
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

impl Block for FirFilterHdl {
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
