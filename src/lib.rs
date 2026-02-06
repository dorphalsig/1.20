//! Pure-Rust pYIN implementation.
//!
//! This crate implements the two-stage pYIN algorithm described in:
//! "pYIN: A Fundamental Frequency Estimator Using Probabilistic Threshold Distributions"
//! (Mauch & Dixon). The implementation follows the paper's equations and is designed
//! for streaming PCM input.

mod frb;
mod hmm;
mod midi;
mod pcm;
mod pyin_stage1;
mod viterbi;
mod yin;

use hmm::{HmmParams, ObservationFrame};
use midi::midi_from_hz;
use pcm::parse_pcm_bytes;
use pyin_stage1::{Stage1CandidateFrame, Stage1Config};
use viterbi::ViterbiTracker;

#[derive(Debug, Clone)]
pub enum PyinError {
    InvalidConfig(String),
}

#[derive(Debug, Clone, Copy)]
pub enum BetaPrior {
    Mean10,
    Mean15,
    Mean20,
    Custom { alpha: f32, beta: f32 },
}

impl BetaPrior {
    pub fn alpha_beta(self) -> (f32, f32) {
        match self {
            BetaPrior::Mean10 => (2.0, 18.0),
            BetaPrior::Mean15 => (2.0, 11.333_333),
            BetaPrior::Mean20 => (2.0, 8.0),
            BetaPrior::Custom { alpha, beta } => (alpha, beta),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PyinConfig {
    pub sample_rate_hz: u32,
    pub frame_size: usize,
    pub hop_size: usize,
    pub fmin_hz: f32,
    pub fmax_hz: f32,
    pub beta_prior: BetaPrior,
    pub pa_absolute_min: f32,
    pub return_candidates: bool,
}

impl Default for PyinConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 48_000,
            frame_size: 2048,
            hop_size: 256,
            fmin_hz: 50.0,
            fmax_hz: 1200.0,
            beta_prior: BetaPrior::Mean10,
            pa_absolute_min: 0.01,
            return_candidates: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameEstimate {
    pub frame_index: u64,
    pub time_sec: f64,
    pub f0_hz: Option<f32>,
    pub voiced: bool,
    /// Confidence is derived from the observation probability assigned to the
    /// winning HMM state (voiced or unvoiced). See `hmm::observation_from_candidates`.
    pub confidence: f32,
    pub midi_note: Option<u8>,
    pub candidates: Option<Vec<(f32, f32)>>,
}

pub struct Pyin {
    cfg: PyinConfig,
    pcm_format: PcmFormat,
    sample_buffer: Vec<f32>,
    leftover_bytes: Vec<u8>,
    stage1_frames: Vec<Stage1CandidateFrame>,
    observation_frames: Vec<ObservationFrame>,
    viterbi: ViterbiTracker,
    last_emitted: usize,
}

impl Pyin {
    pub fn new(cfg: PyinConfig, pcm_format: PcmFormat) -> Result<Self, PyinError> {
        if cfg.frame_size == 0 || cfg.hop_size == 0 {
            return Err(PyinError::InvalidConfig(
                "frame_size and hop_size must be > 0".to_string(),
            ));
        }
        let hmm_params = HmmParams::new();
        Ok(Self {
            cfg,
            pcm_format,
            sample_buffer: Vec::new(),
            leftover_bytes: Vec::new(),
            stage1_frames: Vec::new(),
            observation_frames: Vec::new(),
            viterbi: ViterbiTracker::new(hmm_params),
            last_emitted: 0,
        })
    }

    pub fn reset(&mut self) {
        self.sample_buffer.clear();
        self.leftover_bytes.clear();
        self.stage1_frames.clear();
        self.observation_frames.clear();
        self.viterbi = ViterbiTracker::new(HmmParams::new());
        self.last_emitted = 0;
    }

    pub fn push_bytes(&mut self, chunk: &[u8]) -> Result<Vec<FrameEstimate>, PyinError> {
        let new_samples = parse_pcm_bytes(chunk, self.pcm_format, &mut self.leftover_bytes);
        self.sample_buffer.extend_from_slice(&new_samples);

        let stage1_cfg = Stage1Config::from_config(&self.cfg);

        while self.sample_buffer.len() >= self.cfg.frame_size {
            let frame = &self.sample_buffer[..self.cfg.frame_size];
            let candidate_frame = pyin_stage1::process_frame(frame, &stage1_cfg);
            self.stage1_frames.push(candidate_frame);
            self.sample_buffer.drain(..self.cfg.hop_size.min(self.sample_buffer.len()));
        }

        if self.stage1_frames.is_empty() {
            return Ok(Vec::new());
        }

        while self.observation_frames.len() < self.stage1_frames.len() {
            let frame = &self.stage1_frames[self.observation_frames.len()];
            let obs = hmm::observation_from_candidates(frame);
            self.viterbi.push(&obs);
            self.observation_frames.push(obs);
        }

        let state_path = self.viterbi.best_path();

        let mut output = Vec::new();
        for (idx, state) in state_path.into_iter().enumerate().skip(self.last_emitted) {
            let time_sec = idx as f64 * self.cfg.hop_size as f64 / self.cfg.sample_rate_hz as f64;
            let obs = &self.observation_frames[idx];
            let (f0_hz, voiced, confidence) = if state.voiced {
                let f0 = self.viterbi.params().bin_freqs[state.bin];
                let conf = obs.p_star[state.bin];
                (Some(f0), true, conf)
            } else {
                let conf = 1.0 - obs.sum_p;
                (None, false, conf)
            };
            let midi_note = f0_hz.map(midi_from_hz);
            let candidates = if self.cfg.return_candidates {
                Some(
                    self.stage1_frames[idx]
                        .candidates
                        .iter()
                        .map(|c| (c.frequency_hz, c.probability))
                        .collect(),
                )
            } else {
                None
            };

            output.push(FrameEstimate {
                frame_index: idx as u64,
                time_sec,
                f0_hz,
                voiced,
                confidence,
                midi_note,
                candidates,
            });
        }

        self.last_emitted = self.stage1_frames.len();
        Ok(output)
    }
}

pub use pcm::PcmFormat;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_conversion_basics() {
        assert_eq!(midi_from_hz(440.0), 69);
        assert_eq!(midi_from_hz(880.0), 81);
        assert_eq!(midi_from_hz(220.0), 57);
    }
}


pub use frb::{init_logging, new_processor, push_and_get_midi, PyinProcessor};
