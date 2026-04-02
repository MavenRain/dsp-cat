//! Delay line HDL module.
//!
//! A FIFO circular buffer that delays the input signal by a fixed
//! number of samples.  Synthesizes to BRAM/URAM on FPGA.

use rust_hdl::prelude::*;

use crate::hdl::common::{bits_to_i32, i32_to_bits, SAMPLE_WIDTH};

/// Delay line with configurable depth.
///
/// Latency: `depth` clock cycles.
#[derive(Clone, Debug)]
pub struct DelayLineHdl {
    /// Clock input.
    pub clock: Signal<In, Clock>,
    /// Sample input.
    pub data_in: Signal<In, Bits<SAMPLE_WIDTH>>,
    /// Input valid strobe.
    pub valid_in: Signal<In, Bit>,
    /// Delayed sample output.
    pub data_out: Signal<Out, Bits<SAMPLE_WIDTH>>,
    /// Output valid strobe.
    pub valid_out: Signal<Out, Bit>,
    buffer: Vec<i32>,
    write_ptr: usize,
    depth: usize,
    fill_count: usize,
}

impl DelayLineHdl {
    /// Create a delay line with the given depth.
    #[must_use]
    pub fn new(depth: usize) -> Self {
        Self {
            clock: Signal::default(),
            data_in: Signal::default(),
            valid_in: Signal::default(),
            data_out: Signal::default(),
            valid_out: Signal::default(),
            buffer: vec![0_i32; depth.max(1)],
            write_ptr: 0,
            depth,
            fill_count: 0,
        }
    }
}

impl Default for DelayLineHdl {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Logic for DelayLineHdl {
    fn update(&mut self) {
        if self.valid_in.val() {
            let read_val = self.buffer.get(self.write_ptr).copied().unwrap_or(0);
            self.data_out.next = i32_to_bits(read_val);

            let input_val = bits_to_i32(self.data_in.val());
            if let Some(slot) = self.buffer.get_mut(self.write_ptr) {
                *slot = input_val;
            }
            self.write_ptr = (self.write_ptr + 1) % self.depth.max(1);

            if self.fill_count >= self.depth {
                self.valid_out.next = true;
            } else {
                self.fill_count += 1;
                self.valid_out.next = false;
            }
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

impl Block for DelayLineHdl {
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
