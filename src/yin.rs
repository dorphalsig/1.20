use rustfft::{num_complex::Complex, FftPlanner};

/// Compute the YIN difference function d(τ) for τ in [0, max_tau].
///
/// This uses an FFT-based autocorrelation to avoid the O(N·τ) nested loop,
/// then applies: d(τ) = Σ_{j=0..N-τ-1} x_j^2 + Σ_{j=0..N-τ-1} x_{j+τ}^2 - 2·r(τ).
pub fn difference_function(frame: &[f32], max_tau: usize) -> Vec<f32> {
    let n = frame.len();
    let mut diff = vec![0.0; max_tau + 1];
    if n == 0 || max_tau == 0 {
        return diff;
    }

    let fft_len = (n * 2).next_power_of_two();
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);
    let ifft = planner.plan_fft_inverse(fft_len);

    let mut buffer = vec![Complex { re: 0.0_f32, im: 0.0_f32 }; fft_len];
    for (idx, &sample) in frame.iter().enumerate() {
        buffer[idx].re = sample;
    }

    fft.process(&mut buffer);
    for value in buffer.iter_mut() {
        let power = value.re * value.re + value.im * value.im;
        *value = Complex { re: power, im: 0.0 };
    }
    ifft.process(&mut buffer);

    let scale = 1.0 / fft_len as f32;
    let mut prefix_sq = vec![0.0_f32; n + 1];
    for (idx, &sample) in frame.iter().enumerate() {
        prefix_sq[idx + 1] = prefix_sq[idx] + sample * sample;
    }

    for tau in 1..=max_tau.min(n - 1) {
        let sum_head = prefix_sq[n - tau];
        let sum_tail = prefix_sq[n] - prefix_sq[tau];
        let autocorr = buffer[tau].re * scale;
        diff[tau] = sum_head + sum_tail - 2.0 * autocorr;
    }

    diff
}

/// Compute the cumulative mean normalized difference function d'(τ).
///
/// d'(τ) = d(τ) / ((1/τ) * Σ_{j=1..τ} d(j))
pub fn cumulative_mean_normalized_difference(diff: &[f32]) -> Vec<f32> {
    let mut cmnd = vec![0.0; diff.len()];
    cmnd[0] = 1.0;
    let mut running_sum = 0.0;
    for tau in 1..diff.len() {
        running_sum += diff[tau];
        if running_sum == 0.0 {
            cmnd[tau] = 1.0;
        } else {
            cmnd[tau] = diff[tau] * tau as f32 / running_sum;
        }
    }
    cmnd
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

/// Parabolic interpolation around a minimum to refine τ.
/// Returns the refined τ in floating point.
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
        let diff = difference_function(&frame, 32);
        let cmnd = cumulative_mean_normalized_difference(&diff);
        assert!(cmnd.iter().skip(1).all(|v| (*v - 1.0).abs() < 1e-6));
    }

    #[test]
    fn parabolic_interpolation_minimum() {
        // Quadratic around tau=5 with minimum at 5.2
        let mut cmnd = vec![0.0_f32; 10];
        for i in 0..cmnd.len() {
            let x = i as f32 - 5.2;
            cmnd[i] = x * x;
        }
        let refined = parabolic_interpolation(&cmnd, 5);
        assert!((refined - 5.2).abs() < 0.2);
    }
}
