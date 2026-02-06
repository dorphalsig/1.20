use crate::pyin_stage1::Stage1CandidateFrame;

// Extended beyond the paper's A5 upper bound to support higher notes (e.g. C6) in live use.
pub const NUM_BINS: usize = 600;
const BIN_START_HZ: f32 = 55.0;
const BIN_CENTS: f32 = 10.0;
const MAX_PITCH_JUMP: i32 = 25;

#[derive(Debug, Clone)]
pub struct HmmParams {
    pub bin_freqs: Vec<f32>,
    pub log_pitch_transition: Vec<f32>,
    pub log_voicing_stay: f32,
    pub log_voicing_switch: f32,
}

impl HmmParams {
    pub fn new() -> Self {
        let bin_freqs = (0..NUM_BINS)
            .map(|i| {
                let semitones = (i as f32 * BIN_CENTS) / 100.0;
                BIN_START_HZ * 2.0_f32.powf(semitones / 12.0)
            })
            .collect();
        let log_pitch_transition = pitch_transition_log_probs();
        Self {
            bin_freqs,
            log_pitch_transition,
            log_voicing_stay: 0.99_f32.ln(),
            log_voicing_switch: 0.01_f32.ln(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObservationFrame {
    pub p_star: Vec<f32>,
    pub sum_p: f32,
}

pub fn observation_from_candidates(frame: &Stage1CandidateFrame) -> ObservationFrame {
    let mut p_star = vec![0.0_f32; NUM_BINS];
    for candidate in frame.candidates.iter() {
        if let Some(bin) = freq_to_bin(candidate.frequency_hz) {
            p_star[bin] += candidate.probability;
        }
    }
    let mut sum_p: f32 = p_star.iter().sum();
    if sum_p > 1.0 {
        // Clamp to avoid negative unvoiced probability due to numerical accumulation.
        sum_p = 1.0;
    }
    // pYIN Stage 2 observation model (Eq. 6): p_{m,v} is split equally
    // between voiced and unvoiced, with unvoiced sharing the remaining mass.
    ObservationFrame { p_star, sum_p }
}

pub fn freq_to_bin(freq_hz: f32) -> Option<usize> {
    if freq_hz < BIN_START_HZ {
        return None;
    }
    let cents = 1200.0 * (freq_hz / BIN_START_HZ).log2();
    let bin = (cents / BIN_CENTS).round() as i32;
    if bin >= 0 && (bin as usize) < NUM_BINS {
        Some(bin as usize)
    } else {
        None
    }
}

fn pitch_transition_log_probs() -> Vec<f32> {
    let mut weights = Vec::new();
    let max_delta = MAX_PITCH_JUMP;
    let mut sum = 0.0;
    for delta in -max_delta..=max_delta {
        let weight = (max_delta + 1 - delta.abs()) as f32;
        weights.push(weight);
        sum += weight;
    }
    for w in weights.iter_mut() {
        *w = (*w / sum).ln();
    }
    weights
}

pub fn delta_index(delta: i32) -> Option<usize> {
    let max_delta = MAX_PITCH_JUMP;
    if delta < -max_delta || delta > max_delta {
        return None;
    }
    Some((delta + max_delta) as usize)
}
