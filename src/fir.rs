use std::f32::consts::PI;
use nih_plug::nih_log;

pub struct FIRFilter {
    pub coefficients: Vec<f32>,    // FIR filter coefficients
    delay_line: Vec<f32>,      // Internal buffer for state management
    write_pos: usize,          // Current write position for the delay line
}

impl Default for FIRFilter {
    fn default() -> Self {
        FIRFilter {
            delay_line: vec![0.0],
            coefficients: vec![1.0],
            write_pos: 0,
        }
    }
}

impl FIRFilter {
    pub fn apply(&mut self, input_sample: f32) -> f32 {
        let len = self.coefficients.len();

        // Add the input sample to the delay line
        self.delay_line[self.write_pos] = input_sample;

        // Compute the output sample using convolution
        let mut output = 0.0;
        for (i, &coeff) in self.coefficients.iter().enumerate() {
            let idx = (self.write_pos + len - i) % len; // Circular buffer indexing
            output += coeff * self.delay_line[idx];
        }

        // Update the write position
        self.write_pos = (self.write_pos + 1) % len;

        // Return the computed output sample
        output
    }
    pub fn new(coefficients: Vec<f32>) -> Self {
        FIRFilter {
            delay_line: vec![0.0; coefficients.len()],
            coefficients,
            write_pos: 0,
        }
    }
    pub fn set_coefficient(&mut self, coeff_nr: usize, coeff_value: f32) -> Result<(), &'static str> {
        if coeff_nr < self.coefficients.len() {
            self.coefficients[coeff_nr] = coeff_value;
            Ok(())
        } else {
            Err("Coefficient index out of bounds")
        }
    }
    pub fn set_coefficients_for_lowpass(&mut self, cutoff: f32) -> Result<(), &'static str> {
        if cutoff < 1.0 {
            let nr_coeffs = self.coefficients.len();
            let order = nr_coeffs - 1;
            let m = order as f32 / 2.0;

            // calculate coefficients
            for n in 0..=order {
                let sinc_val = sinc(2.0 * cutoff * (n as f32 - m));
                let window_val = hamming_window(n, order);
                self.coefficients[n] = sinc_val * window_val;
            }

            // Normalize coefficients to ensure unity gain at DC
            let sum: f32 = self.coefficients.iter().sum();
            self.coefficients.iter_mut().for_each(|c| *c /= sum);

            Ok(())
        } else {
            Err("Cutoff out of bounds (should be < 1.0)")
        }
    }
    fn reset(&mut self) {
        self.delay_line.fill(0.0);
        self.write_pos = 0;
    }
    pub fn len(&self) -> usize {
        self.coefficients.len()
    }
}

pub fn sinc(x: f32) -> f32 {
    if x.abs() < f32::EPSILON {
        1.0
    } else {
        (x * PI).sin() / (x * PI)
    }
}

pub fn hamming_window(n: usize, order: usize) -> f32 {
    0.54 - 0.46 * ((2.0 * PI * n as f32) / order as f32).cos()
}

// cutoff is the normalized frequency, 0 < cutoff < 0.5
// where 1.0 is the sampling rate
pub fn fir_lowpass_coefficients(cutoff: f32, order: usize) -> Vec<f32> {
    let m = order as f32 / 2.0;
    let mut coefficients = Vec::with_capacity(order + 1);

    for n in 0..=order {
        let sinc_val = sinc(2.0 * cutoff * (n as f32 - m));
        let window_val = hamming_window(n, order);
        coefficients.push(sinc_val * window_val);
    }

    // Normalize coefficients to ensure unity gain at DC
    let sum: f32 = coefficients.iter().sum();
    coefficients.iter_mut().for_each(|c| *c /= sum);

    coefficients
}
