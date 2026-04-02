//! Coefficient tap chain (shift register) for FIR filter.
//!
//! Stores the filter coefficients and delays the input sample
//! through a shift register so each MAC unit sees the correct
//! historical sample.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, SAMPLE_WIDTH};

/// Shift register that delays samples for FIR tap alignment.
///
/// Each clock cycle with valid input, the newest sample enters
/// position 0 and all existing samples shift one position deeper.
#[derive(Clone, Debug)]
pub struct TapChain {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// New sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    taps: Vec<i32>,
    tap_count: usize,
}

impl TapChain {
    /// Create a tap chain of the given length.
    #[must_use]
    pub fn new(tap_count: usize) -> Self {
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            taps: vec![0_i32; tap_count],
            tap_count,
        }
    }

    /// Read the sample at the given tap index.
    #[must_use]
    pub fn tap_value(&self, index: usize) -> i32 {
        self.taps.get(index).copied().unwrap_or(0)
    }

    /// The number of taps.
    #[must_use]
    pub fn tap_count(&self) -> usize {
        self.tap_count
    }
}

impl Default for TapChain {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Logic for TapChain {
    fn update(&mut self) {
        if self.valid_in.val() {
            // Shift all taps one position deeper
            (1..self.tap_count).rev().for_each(|k| {
                let prev = self.taps.get(k - 1).copied().unwrap_or(0);
                if let Some(slot) = self.taps.get_mut(k) {
                    *slot = prev;
                }
            });
            // Insert new sample at position 0
            if let Some(slot) = self.taps.first_mut() {
                *slot = bits_to_i32(self.data_in.val());
            }
        }
    }

    fn connect(&mut self) {}

    fn hdl(&self) -> Verilog {
        Verilog::Empty
    }
}

impl Block for TapChain {
    fn connect_all(&mut self) {
        self.connect();
    }

    fn update_all(&mut self) {
        self.update();
    }

    fn has_changed(&self) -> bool {
        false
    }

    fn accept(&self, name: &str, probe: &mut dyn Probe) {
        probe.visit_start_scope(name, self);
        probe.visit_end_scope(name, self);
    }
}
