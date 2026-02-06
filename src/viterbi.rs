use crate::hmm::{delta_index, HmmParams, ObservationFrame, NUM_BINS};

#[derive(Debug, Clone, Copy)]
pub struct HmmState {
    pub bin: usize,
    pub voiced: bool,
}

pub struct ViterbiTracker {
    params: HmmParams,
    backpointers: Vec<Vec<usize>>,
    prev_scores: Vec<f32>,
    frames: usize,
}

impl ViterbiTracker {
    pub fn new(params: HmmParams) -> Self {
        let num_states = NUM_BINS * 2;
        Self {
            params,
            backpointers: Vec::new(),
            prev_scores: vec![f32::NEG_INFINITY; num_states],
            frames: 0,
        }
    }

    pub fn push(&mut self, obs: &ObservationFrame) {
        let num_states = NUM_BINS * 2;
        let mut curr = vec![f32::NEG_INFINITY; num_states];
        let mut back = vec![0; num_states];

        if self.frames == 0 {
            let log_init = (1.0 / NUM_BINS as f32).ln();
            let unvoiced_log = safe_log(0.5 * (1.0 - obs.sum_p));
            for bin in 0..NUM_BINS {
                let idx = state_index(bin, false);
                self.prev_scores[idx] = log_init + unvoiced_log;
            }
        } else {
            let unvoiced_log = safe_log(0.5 * (1.0 - obs.sum_p));
            for next_bin in 0..NUM_BINS {
                let voiced_log = safe_log(0.5 * obs.p_star[next_bin]);

                for &next_voiced in &[false, true] {
                    let obs_log = if next_voiced { voiced_log } else { unvoiced_log };
                    let mut best_prev = f32::NEG_INFINITY;
                    let mut best_state = 0;
                    let min_prev = next_bin.saturating_sub(25);
                    let max_prev = (next_bin + 25).min(NUM_BINS - 1);
                    for prev_bin in min_prev..=max_prev {
                        let delta = next_bin as i32 - prev_bin as i32;
                        let pitch_log = self.params.log_pitch_transition[delta_index(delta).unwrap()];
                        for &prev_voiced in &[false, true] {
                            // Eq. (7): voicing transition. Eq. (8): triangular pitch transition.
                            let voicing_log = if prev_voiced == next_voiced {
                                self.params.log_voicing_stay
                            } else {
                                self.params.log_voicing_switch
                            };
                            let prev_idx = state_index(prev_bin, prev_voiced);
                            let score = self.prev_scores[prev_idx] + pitch_log + voicing_log;
                            if score > best_prev {
                                best_prev = score;
                                best_state = prev_idx;
                            }
                        }
                    }
                    let idx = state_index(next_bin, next_voiced);
                    curr[idx] = best_prev + obs_log;
                    back[idx] = best_state;
                }
            }
            self.prev_scores = curr;
        }

        self.backpointers.push(back);
        self.frames += 1;
    }

    pub fn best_path(&self) -> Vec<HmmState> {
        if self.frames == 0 {
            return Vec::new();
        }
        let mut best_final = 0;
        let mut best_score = f32::NEG_INFINITY;
        for (idx, score) in self.prev_scores.iter().enumerate() {
            if *score > best_score {
                best_score = *score;
                best_final = idx;
            }
        }

        let mut path = vec![best_final; self.frames];
        for t in (1..self.frames).rev() {
            path[t - 1] = self.backpointers[t][path[t]];
        }

        path.into_iter().map(state_from_index).collect()
    }

    pub fn params(&self) -> &HmmParams {
        &self.params
    }
}

fn state_index(bin: usize, voiced: bool) -> usize {
    if voiced {
        NUM_BINS + bin
    } else {
        bin
    }
}

fn state_from_index(idx: usize) -> HmmState {
    if idx >= NUM_BINS {
        HmmState {
            bin: idx - NUM_BINS,
            voiced: true,
        }
    } else {
        HmmState { bin: idx, voiced: false }
    }
}

fn safe_log(prob: f32) -> f32 {
    // Numerical stability: avoid log(0) while preserving relative scoring.
    const FLOOR: f32 = 1e-12;
    prob.max(FLOOR).ln()
}
