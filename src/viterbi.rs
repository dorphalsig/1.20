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
    buffer_curr: Vec<f32>,
    buffer_back: Vec<usize>,
}

impl ViterbiTracker {
    pub fn new(params: HmmParams) -> Self {
        let num_states = NUM_BINS * 2;
        Self {
            params,
            backpointers: Vec::new(),
            prev_scores: vec![f32::NEG_INFINITY; num_states],
            frames: 0,
            buffer_curr: vec![f32::NEG_INFINITY; num_states],
            buffer_back: vec![0; num_states],
        }
    }

    pub fn push(&mut self, obs: &ObservationFrame) {
        let num_states = NUM_BINS * 2;
        if self.buffer_curr.len() != num_states {
            self.buffer_curr.resize(num_states, f32::NEG_INFINITY);
        }
        if self.buffer_back.len() != num_states {
            self.buffer_back.resize(num_states, 0);
        }
        self.buffer_curr.fill(f32::NEG_INFINITY);
        self.buffer_back.fill(0);

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
                    let obs_log = if next_voiced {
                        voiced_log
                    } else {
                        unvoiced_log
                    };
                    let mut best_prev = f32::NEG_INFINITY;
                    let mut best_state = 0;
                    let min_prev = next_bin.saturating_sub(25);
                    let max_prev = (next_bin + 25).min(NUM_BINS - 1);
                    for prev_bin in min_prev..=max_prev {
                        let delta = next_bin as i32 - prev_bin as i32;
                        let pitch_log =
                            self.params.log_pitch_transition[delta_index(delta).unwrap()];
                        for &prev_voiced in &[false, true] {
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
                    self.buffer_curr[idx] = best_prev + obs_log;
                    self.buffer_back[idx] = best_state;
                }
            }
            std::mem::swap(&mut self.prev_scores, &mut self.buffer_curr);
        }

        self.backpointers.push(self.buffer_back.clone());
        self.frames += 1;
    }

    pub fn best_suffix_from(&self, start_frame: usize) -> Vec<HmmState> {
        if self.frames == 0 || start_frame >= self.frames {
            return Vec::new();
        }

        let mut best_final = 0usize;
        let mut best_score = f32::NEG_INFINITY;
        for (idx, score) in self.prev_scores.iter().enumerate() {
            if *score > best_score {
                best_score = *score;
                best_final = idx;
            }
        }

        let mut state = best_final;
        let mut rev = Vec::with_capacity(self.frames - start_frame);
        for t in (start_frame..self.frames).rev() {
            rev.push(state_from_index(state));
            if t > 0 {
                state = self.backpointers[t][state];
            }
        }
        rev.reverse();
        rev
    }

    pub fn prune(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let prune = count.min(self.backpointers.len());
        if prune == 0 {
            return;
        }
        self.backpointers.drain(0..prune);
        self.frames = self.frames.saturating_sub(prune);
    }

    pub fn best_path(&self) -> Vec<HmmState> {
        self.best_suffix_from(0)
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
    const FLOOR: f32 = 1e-12;
    prob.max(FLOOR).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs_with_bin(bin: usize) -> ObservationFrame {
        let mut p_star = vec![0.0; NUM_BINS];
        p_star[bin] = 1.0;
        ObservationFrame { p_star, sum_p: 1.0 }
    }

    #[test]
    fn best_path_tracks_clear_transition() {
        let mut tracker = ViterbiTracker::new(HmmParams::new());
        let bin_a = 100;
        let bin_b = 110;
        for _ in 0..3 {
            tracker.push(&obs_with_bin(bin_a));
        }
        for _ in 0..3 {
            tracker.push(&obs_with_bin(bin_b));
        }
        let path = tracker.best_path();
        assert_eq!(path.len(), 6);
        for state in &path[..3] {
            assert_eq!(state.bin, bin_a);
        }
        for state in &path[3..] {
            assert_eq!(state.bin, bin_b);
        }
    }
}
