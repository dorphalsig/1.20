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
use pcm::parse_pcm_bytes_into;
use pyin_stage1::{process_frame_with_scratch, Stage1CandidateFrame, Stage1Config};
use viterbi::ViterbiTracker;
use yin::YinScratch;

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
    pub confidence: f32,
    pub midi_note: Option<u8>,
    pub candidates: Option<Vec<(f32, f32)>>,
}

pub struct Pyin {
    cfg: PyinConfig,
    pcm_format: PcmFormat,
    sample_buffer: Vec<f32>,
    sample_start: usize,
    leftover_bytes: Vec<u8>,
    stage1_frames: Vec<Stage1CandidateFrame>,
    observation_frames: Vec<ObservationFrame>,
    viterbi: ViterbiTracker,
    last_emitted: usize,
    stage1_cfg: Stage1Config,
    yin_scratch: YinScratch,
}

impl Pyin {
    pub fn new(cfg: PyinConfig, pcm_format: PcmFormat) -> Result<Self, PyinError> {
        if cfg.frame_size == 0 || cfg.hop_size == 0 {
            return Err(PyinError::InvalidConfig(
                "frame_size and hop_size must be > 0".to_string(),
            ));
        }
        let hmm_params = HmmParams::new();
        let stage1_cfg = Stage1Config::from_config(&cfg);
        let fft_len = (cfg.frame_size.saturating_mul(2))
            .next_power_of_two()
            .max(2);
        Ok(Self {
            cfg,
            pcm_format,
            sample_buffer: Vec::new(),
            sample_start: 0,
            leftover_bytes: Vec::new(),
            stage1_frames: Vec::new(),
            observation_frames: Vec::new(),
            viterbi: ViterbiTracker::new(hmm_params),
            last_emitted: 0,
            stage1_cfg,
            yin_scratch: YinScratch::new(fft_len),
        })
    }

    pub fn reset(&mut self) {
        self.sample_buffer.clear();
        self.sample_start = 0;
        self.leftover_bytes.clear();
        self.stage1_frames.clear();
        self.observation_frames.clear();
        self.viterbi = ViterbiTracker::new(HmmParams::new());
        self.last_emitted = 0;
    }

    pub fn push_bytes(&mut self, chunk: &[u8]) -> Result<Vec<FrameEstimate>, PyinError> {
        parse_pcm_bytes_into(
            chunk,
            self.pcm_format,
            &mut self.leftover_bytes,
            &mut self.sample_buffer,
        );

        while self.sample_buffer.len().saturating_sub(self.sample_start) >= self.cfg.frame_size {
            let frame =
                &self.sample_buffer[self.sample_start..self.sample_start + self.cfg.frame_size];
            let candidate_frame =
                process_frame_with_scratch(frame, &self.stage1_cfg, &mut self.yin_scratch);
            self.stage1_frames.push(candidate_frame);
            self.sample_start += self.cfg.hop_size;
        }

        self.compact_sample_buffer_if_needed();

        if self.stage1_frames.is_empty() {
            return Ok(Vec::new());
        }

        while self.observation_frames.len() < self.stage1_frames.len() {
            let frame = &self.stage1_frames[self.observation_frames.len()];
            let obs = hmm::observation_from_candidates(frame);
            self.viterbi.push(&obs);
            self.observation_frames.push(obs);
        }

        let new_states = self.viterbi.best_suffix_from(self.last_emitted);
        let mut output = Vec::with_capacity(new_states.len());
        for (offset, state) in new_states.into_iter().enumerate() {
            let idx = self.last_emitted + offset;
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

    fn compact_sample_buffer_if_needed(&mut self) {
        let threshold = self.cfg.frame_size.saturating_mul(4).max(8192);
        if self.sample_start <= threshold {
            return;
        }
        self.sample_buffer.copy_within(self.sample_start.., 0);
        self.sample_buffer
            .truncate(self.sample_buffer.len() - self.sample_start);
        self.sample_start = 0;
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
