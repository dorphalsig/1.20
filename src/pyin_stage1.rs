use crate::{BetaPrior, PyinConfig};
use crate::yin::{cumulative_mean_normalized_difference, difference_function, local_minima, parabolic_interpolation};

#[derive(Debug, Clone)]
pub struct Stage1Config {
    pub sample_rate_hz: u32,
    pub frame_size: usize,
    pub fmin_hz: f32,
    pub fmax_hz: f32,
    pub thresholds: Vec<f32>,
    pub threshold_priors: Vec<f32>,
    pub pa_absolute_min: f32,
}

impl Stage1Config {
    pub fn from_config(cfg: &PyinConfig) -> Self {
        let thresholds: Vec<f32> = (1..=100).map(|i| i as f32 * 0.01).collect();
        let threshold_priors = beta_prior_distribution(&thresholds, cfg.beta_prior);
        Self {
            sample_rate_hz: cfg.sample_rate_hz,
            frame_size: cfg.frame_size,
            fmin_hz: cfg.fmin_hz,
            fmax_hz: cfg.fmax_hz,
            thresholds,
            threshold_priors,
            pa_absolute_min: cfg.pa_absolute_min,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub frequency_hz: f32,
    pub probability: f32,
}

#[derive(Debug, Clone)]
pub struct Stage1CandidateFrame {
    pub candidates: Vec<Candidate>,
}

pub fn process_frame(frame: &[f32], cfg: &Stage1Config) -> Stage1CandidateFrame {
    let max_tau = (cfg.sample_rate_hz as f32 / cfg.fmin_hz).floor() as usize;
    let min_tau = (cfg.sample_rate_hz as f32 / cfg.fmax_hz).ceil() as usize;
    let max_tau = max_tau.min(cfg.frame_size.saturating_sub(1));
    let min_tau = min_tau.max(1).min(max_tau);

    let diff = difference_function(frame, max_tau);
    let cmnd = cumulative_mean_normalized_difference(&diff);

    let minima = local_minima(&cmnd);
    let global_min_tau = (min_tau..=max_tau)
        .min_by(|&a, &b| cmnd[a].partial_cmp(&cmnd[b]).unwrap())
        .unwrap_or(min_tau);

    let mut candidate_map: Vec<(usize, f32)> = Vec::new();

    for (idx, threshold) in cfg.thresholds.iter().enumerate() {
        let mut selected_tau = None;
        for &tau in minima.iter() {
            if tau < min_tau || tau > max_tau {
                continue;
            }
            if cmnd[tau] < *threshold {
                selected_tau = Some(tau);
                break;
            }
        }
        // pYIN Stage 1: Eq. (4) and (5) from the paper.
        // Y(x_t, s_i) returns the smallest local minimum below s_i; otherwise
        // we fall back to the global minimum with the absolute-minimum strategy
        // weight pa (a(s_i, τ) = pa).
        let (tau, a_weight) = if let Some(tau) = selected_tau {
            (tau, 1.0)
        } else {
            (global_min_tau, cfg.pa_absolute_min)
        };
        let prior = cfg.threshold_priors[idx];
        let weight = a_weight * prior;
        if let Some(entry) = candidate_map.iter_mut().find(|(t, _)| *t == tau) {
            entry.1 += weight;
        } else {
            candidate_map.push((tau, weight));
        }
    }

    let mut candidates = Vec::new();
    for (tau, prob) in candidate_map {
        let refined_tau = parabolic_interpolation(&cmnd, tau).max(1.0);
        let frequency = cfg.sample_rate_hz as f32 / refined_tau;
        candidates.push(Candidate {
            frequency_hz: frequency,
            probability: prob,
        });
    }

    Stage1CandidateFrame { candidates }
}

/// Compute discrete beta prior weights over thresholds.
fn beta_prior_distribution(thresholds: &[f32], prior: BetaPrior) -> Vec<f32> {
    let (alpha, beta) = prior.alpha_beta();
    let mut weights: Vec<f32> = thresholds
        .iter()
        .map(|&s| {
            if s <= 0.0 || s >= 1.0 {
                0.0
            } else {
                s.powf(alpha - 1.0) * (1.0 - s).powf(beta - 1.0)
            }
        })
        .collect();
    let sum: f32 = weights.iter().sum();
    if sum > 0.0 {
        for w in weights.iter_mut() {
            *w /= sum;
        }
    }
    weights
}
