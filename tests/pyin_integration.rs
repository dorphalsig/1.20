use pyin_rs::{PcmFormat, Pyin, PyinConfig};
use rand::{rngs::StdRng, Rng, SeedableRng};

fn sine_wave(freq_hz: f32, duration_sec: f32, sample_rate: u32) -> Vec<f32> {
    let len = (duration_sec * sample_rate as f32) as usize;
    (0..len)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq_hz * t).sin()
        })
        .collect()
}

fn silence(duration_sec: f32, sample_rate: u32) -> Vec<f32> {
    let len = (duration_sec * sample_rate as f32) as usize;
    vec![0.0; len]
}

fn samples_to_i16le(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for &s in samples.iter() {
        let clamped = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
        bytes.extend_from_slice(&clamped.to_le_bytes());
    }
    bytes
}

fn run_stream(cfg: PyinConfig, samples: &[f32]) -> Vec<f32> {
    let mut pyin = Pyin::new(cfg, PcmFormat::I16LE).unwrap();
    let bytes = samples_to_i16le(samples);
    let mut rng = StdRng::seed_from_u64(42);
    let mut idx = 0;
    let mut f0s = Vec::new();
    while idx < bytes.len() {
        let chunk_size = rng.gen_range(1..2048).min(bytes.len() - idx);
        let chunk = &bytes[idx..idx + chunk_size];
        let frames = pyin.push_bytes(chunk).unwrap();
        for frame in frames {
            f0s.push(frame.f0_hz.unwrap_or(0.0));
        }
        idx += chunk_size;
    }
    f0s
}


fn median_cents_error(estimates: &[f32], target: f32) -> f32 {
    let mut errors: Vec<f32> = estimates
        .iter()
        .filter(|v| **v > 0.0)
        .map(|v| 1200.0 * (v / target).log2().abs())
        .collect();
    errors.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if errors.is_empty() {
        return 0.0;
    }
    errors[errors.len() / 2]
}

#[test]
fn single_sine_tones_accuracy() {
    let cfg = PyinConfig::default();
    for &freq in &[110.0, 220.0, 440.0, 523.25] {
        let samples = sine_wave(freq, 2.5, cfg.sample_rate_hz);
        let estimates = run_stream(cfg.clone(), &samples);
        let voiced_ratio = estimates.iter().filter(|v| **v > 0.0).count() as f32
            / estimates.len().max(1) as f32;
        assert!(voiced_ratio > 0.8);
        let median_error = median_cents_error(&estimates, freq);
        assert!(median_error < 25.0);
    }
}

#[test]
fn voicing_detection_with_silence() {
    let cfg = PyinConfig::default();
    let mut samples = sine_wave(220.0, 1.0, cfg.sample_rate_hz);
    samples.extend_from_slice(&silence(1.0, cfg.sample_rate_hz));
    samples.extend_from_slice(&sine_wave(220.0, 1.0, cfg.sample_rate_hz));
    let estimates = run_stream(cfg.clone(), &samples);

    let frames_per_sec = estimates.len() / 3;
    let first = &estimates[..frames_per_sec];
    let middle = &estimates[frames_per_sec..2 * frames_per_sec];
    let last = &estimates[2 * frames_per_sec..];

    let voiced_first = first.iter().filter(|v| **v > 0.0).count() as f32 / first.len() as f32;
    let voiced_middle = middle.iter().filter(|v| **v > 0.0).count() as f32 / middle.len() as f32;
    let voiced_last = last.iter().filter(|v| **v > 0.0).count() as f32 / last.len() as f32;

    assert!(voiced_first > 0.8);
    assert!(voiced_middle < 0.2);
    assert!(voiced_last > 0.8);
}

#[test]
fn pitch_contour_tracking_steps() {
    let cfg = PyinConfig::default();
    let mut samples = Vec::new();
    samples.extend_from_slice(&sine_wave(220.0, 0.5, cfg.sample_rate_hz));
    samples.extend_from_slice(&silence(0.1, cfg.sample_rate_hz));
    samples.extend_from_slice(&sine_wave(247.0, 0.5, cfg.sample_rate_hz));
    samples.extend_from_slice(&silence(0.1, cfg.sample_rate_hz));
    samples.extend_from_slice(&sine_wave(262.0, 0.5, cfg.sample_rate_hz));

    let estimates = run_stream(cfg.clone(), &samples);

    let frames_per_step = estimates.len() / 5;
    let step1 = &estimates[..frames_per_step];
    let step2 = &estimates[frames_per_step * 2..frames_per_step * 3];
    let step3 = &estimates[frames_per_step * 4..];

    let err1 = median_cents_error(step1, 220.0);
    let err2 = median_cents_error(step2, 247.0);
    let err3 = median_cents_error(step3, 262.0);

    assert!(err1 < 50.0);
    assert!(err2 < 50.0);
    assert!(err3 < 50.0);
}

fn correctness_metrics(est: &[Option<f32>], truth: &[Option<f32>]) -> (f32, f32, f32) {
    let mut correct = 0;
    let mut true_voiced = 0;
    let mut pred_voiced = 0;

    for (e, t) in est.iter().zip(truth.iter()) {
        let is_true_voiced = t.is_some();
        let is_pred_voiced = e.is_some();
        if is_true_voiced {
            true_voiced += 1;
        }
        if is_pred_voiced {
            pred_voiced += 1;
        }
        if let (Some(eh), Some(th)) = (e, t) {
            let cents = 1200.0 * (eh / th).log2().abs();
            if cents <= 100.0 {
                correct += 1;
            }
        }
    }

    let recall = if true_voiced > 0 {
        correct as f32 / true_voiced as f32
    } else {
        0.0
    };
    let precision = if pred_voiced > 0 {
        correct as f32 / pred_voiced as f32
    } else {
        0.0
    };
    let f = if recall + precision > 0.0 {
        2.0 * recall * precision / (recall + precision)
    } else {
        0.0
    };
    (recall, precision, f)
}

#[test]
fn paper_style_metric_on_synthetic_melody() {
    let cfg = PyinConfig::default();
    let melody = [220.0, 247.0, 262.0, 294.0, 330.0, 349.0, 392.0, 440.0];
    let note_dur = 0.25;

    let mut samples = Vec::new();
    let mut truth = Vec::new();
    for &freq in melody.iter() {
        samples.extend_from_slice(&sine_wave(freq, note_dur, cfg.sample_rate_hz));
    }

    let estimates = run_stream(cfg.clone(), &samples);
    let total_frames = estimates.len();
    let frames_per_note = (note_dur * cfg.sample_rate_hz as f32 / cfg.hop_size as f32) as usize;
    for (idx, &freq) in melody.iter().enumerate() {
        let start = idx * frames_per_note;
        let end = (start + frames_per_note).min(total_frames);
        for _ in start..end {
            truth.push(Some(freq));
        }
    }
    truth.resize(total_frames, None);

    let est_opts: Vec<Option<f32>> = estimates
        .iter()
        .map(|v| if *v > 0.0 { Some(*v) } else { None })
        .collect();

    let (_recall, _precision, f) = correctness_metrics(&est_opts, &truth);
    assert!(f > 0.9);
}

#[test]
fn regression_configurations() {
    let mut cfg = PyinConfig::default();
    cfg.frame_size = 2048;
    cfg.hop_size = 256;
    let samples = sine_wave(440.0, 2.0, cfg.sample_rate_hz);
    let estimates = run_stream(cfg.clone(), &samples);
    assert!(median_cents_error(&estimates, 440.0) < 40.0);

    cfg.frame_size = 4096;
    cfg.hop_size = 480;
    let samples = sine_wave(440.0, 2.0, cfg.sample_rate_hz);
    let estimates = run_stream(cfg, &samples);
    assert!(median_cents_error(&estimates, 440.0) < 60.0);
}
