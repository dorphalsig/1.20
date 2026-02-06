use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};

pub struct YinScratch {
    fft_len: usize,
    fft: Arc<dyn Fft<f32>>,
    ifft: Arc<dyn Fft<f32>>,
    fft_buffer: Vec<Complex<f32>>,
    prefix_sq: Vec<f32>,
    diff: Vec<f32>,
    cmnd: Vec<f32>,
}

impl YinScratch {
    pub fn new(fft_len: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(fft_len);
        let ifft = planner.plan_fft_inverse(fft_len);
        Self {
            fft_len,
            fft,
            ifft,
            fft_buffer: vec![Complex { re: 0.0, im: 0.0 }; fft_len],
            prefix_sq: Vec::new(),
            diff: Vec::new(),
            cmnd: Vec::new(),
        }
    }

    pub fn ensure_fft_len(&mut self, fft_len: usize) {
        if self.fft_len == fft_len {
            return;
        }
        let mut planner = FftPlanner::new();
        self.fft = planner.plan_fft_forward(fft_len);
        self.ifft = planner.plan_fft_inverse(fft_len);
        self.fft_len = fft_len;
        self.fft_buffer
            .resize(fft_len, Complex { re: 0.0, im: 0.0 });
    }

    pub fn ensure_frame_capacity(&mut self, frame_len: usize, max_tau: usize) {
        if self.prefix_sq.len() < frame_len + 1 {
            self.prefix_sq.resize(frame_len + 1, 0.0);
        }
        if self.diff.len() < max_tau + 1 {
            self.diff.resize(max_tau + 1, 0.0);
        }
        if self.cmnd.len() < max_tau + 1 {
            self.cmnd.resize(max_tau + 1, 0.0);
        }
    }

    pub fn diff_slice(&self, len: usize) -> &[f32] {
        &self.diff[..len]
    }

    pub fn cmnd_slice_mut(&mut self, len: usize) -> &mut [f32] {
        if self.cmnd.len() < len {
            self.cmnd.resize(len, 0.0);
        }
        &mut self.cmnd[..len]
    }

    pub fn compute_cmnd_from_diff(&mut self, diff_len: usize) -> &[f32] {
        if self.cmnd.len() < diff_len {
            self.cmnd.resize(diff_len, 0.0);
        }
        if diff_len == 0 {
            return &self.cmnd[..0];
        }
        self.cmnd[0] = 1.0;
        let mut running_sum = 0.0;
        for tau in 1..diff_len {
            running_sum += self.diff[tau];
            self.cmnd[tau] = if running_sum == 0.0 {
                1.0
            } else {
                self.diff[tau] * tau as f32 / running_sum
            };
        }
        &self.cmnd[..diff_len]
    }
}

pub fn difference_function(frame: &[f32], max_tau: usize) -> Vec<f32> {
    let fft_len = (frame.len().saturating_mul(2)).next_power_of_two().max(2);
    let mut scratch = YinScratch::new(fft_len);
    let len = difference_function_inplace(frame, max_tau, &mut scratch);
    scratch.diff_slice(len).to_vec()
}

pub fn difference_function_inplace(
    frame: &[f32],
    max_tau: usize,
    scratch: &mut YinScratch,
) -> usize {
    let n = frame.len();
    let needed = max_tau + 1;
    if n == 0 || max_tau == 0 {
        scratch.diff.resize(needed, 0.0);
        scratch.diff.fill(0.0);
        return needed;
    }

    let fft_len = (n * 2).next_power_of_two();
    scratch.ensure_fft_len(fft_len);
    scratch.ensure_frame_capacity(n, max_tau);

    scratch.fft_buffer.fill(Complex { re: 0.0, im: 0.0 });
    for (idx, &sample) in frame.iter().enumerate() {
        scratch.fft_buffer[idx].re = sample;
    }

    scratch.fft.process(&mut scratch.fft_buffer);
    for value in scratch.fft_buffer.iter_mut() {
        let power = value.re * value.re + value.im * value.im;
        *value = Complex { re: power, im: 0.0 };
    }
    scratch.ifft.process(&mut scratch.fft_buffer);

    scratch.prefix_sq[0] = 0.0;
    for (idx, &sample) in frame.iter().enumerate() {
        scratch.prefix_sq[idx + 1] = scratch.prefix_sq[idx] + sample * sample;
    }

    let diff = &mut scratch.diff[..needed];
    diff.fill(0.0);

    let scale = 1.0 / scratch.fft_len as f32;
    for tau in 1..=max_tau.min(n - 1) {
        let sum_head = scratch.prefix_sq[n - tau];
        let sum_tail = scratch.prefix_sq[n] - scratch.prefix_sq[tau];
        let autocorr = scratch.fft_buffer[tau].re * scale;
        let denom = (n - tau) as f32;
        diff[tau] = (sum_head + sum_tail - 2.0 * autocorr) / denom;
    }

    needed
}

pub fn cumulative_mean_normalized_difference(diff: &[f32]) -> Vec<f32> {
    let mut out = vec![0.0; diff.len()];
    cumulative_mean_normalized_difference_inplace(diff, &mut out);
    out
}

pub fn cumulative_mean_normalized_difference_inplace(diff: &[f32], out: &mut [f32]) {
    if diff.is_empty() {
        return;
    }
    out[0] = 1.0;
    let mut running_sum = 0.0;
    for tau in 1..diff.len() {
        running_sum += diff[tau];
        out[tau] = if running_sum == 0.0 {
            1.0
        } else {
            diff[tau] * tau as f32 / running_sum
        };
    }
}

pub fn local_minima(cmnd: &[f32]) -> Vec<usize> {
    let mut minima = Vec::new();
    if cmnd.len() < 3 {
        return minima;
    }
    for tau in 1..(cmnd.len() - 1) {
        if cmnd[tau] < cmnd[tau - 1] && cmnd[tau] <= cmnd[tau + 1] {
            minima.push(tau);
        }
    }
    minima
}

pub fn parabolic_interpolation(cmnd: &[f32], tau: usize) -> f32 {
    if tau == 0 || tau + 1 >= cmnd.len() {
        return tau as f32;
    }
    let y1 = cmnd[tau - 1];
    let y2 = cmnd[tau];
    let y3 = cmnd[tau + 1];
    let denom = y1 - 2.0 * y2 + y3;
    if denom.abs() < 1e-12 {
        return tau as f32;
    }
    let delta = 0.5 * (y1 - y3) / denom;
    tau as f32 + delta
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmnd_constant_signal() {
        let frame = vec![1.0_f32; 64];
        let mut scratch = YinScratch::new((frame.len() * 2).next_power_of_two());
        let len = difference_function_inplace(&frame, 32, &mut scratch);
        let diff = scratch.diff_slice(len);
        let cmnd = cumulative_mean_normalized_difference(diff);
        assert!(cmnd.iter().skip(1).all(|v| (*v - 1.0).abs() < 1e-6));
    }

    #[test]
    fn parabolic_interpolation_minimum() {
        let mut cmnd = vec![0.0_f32; 10];
        for i in 0..cmnd.len() {
            let x = i as f32 - 5.2;
            cmnd[i] = x * x;
        }
        let refined = parabolic_interpolation(&cmnd, 5);
        assert!((refined - 5.2).abs() < 0.2);
    }
}
