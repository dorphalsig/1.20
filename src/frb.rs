use crate::{PcmFormat, Pyin, PyinConfig};
use std::collections::VecDeque;
use std::sync::Once;

const UNVOICED_MIDI: u16 = 255;

pub struct PyinProcessor {
    pyin: Option<Pyin>,
    sample_queue: VecDeque<i16>,
    frame_buf: Vec<i16>,
    carry_byte: Option<u8>,
    frame_size_samples: usize,
    hop_size_samples: usize,
    invalid_config: bool,
}

impl PyinProcessor {
    fn new(sample_rate_hz: u32, window_ms: u32, hop_ms: u32) -> Self {
        let frame_size_samples = ms_to_samples(sample_rate_hz, window_ms);
        let hop_size_samples = ms_to_samples(sample_rate_hz, hop_ms);
        let invalid_config = frame_size_samples < hop_size_samples;

        if invalid_config {
            log::error!(
                "invalid processor config: frame_size_samples={} hop_size_samples={}",
                frame_size_samples,
                hop_size_samples
            );
            return Self {
                pyin: None,
                sample_queue: VecDeque::new(),
                frame_buf: Vec::new(),
                carry_byte: None,
                frame_size_samples,
                hop_size_samples,
                invalid_config,
            };
        }

        let cfg = PyinConfig {
            sample_rate_hz,
            frame_size: frame_size_samples,
            hop_size: hop_size_samples,
            fmin_hz: 40.0,
            fmax_hz: 2_000.0,
            ..PyinConfig::default()
        };

        let pyin = match Pyin::new(cfg, PcmFormat::I16LE) {
            Ok(v) => Some(v),
            Err(err) => {
                log::error!("failed to initialize pyin: {:?}", err);
                None
            }
        };

        Self {
            pyin,
            sample_queue: VecDeque::new(),
            frame_buf: vec![0; frame_size_samples],
            carry_byte: None,
            frame_size_samples,
            hop_size_samples,
            invalid_config,
        }
    }
}

fn ms_to_samples(sample_rate_hz: u32, ms: u32) -> usize {
    let samples = (sample_rate_hz as f64 * ms as f64 / 1000.0).round() as usize;
    samples.max(1)
}

pub fn init_logging() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .is_test(true)
            .try_init();
    });
}

pub fn new_processor(sample_rate_hz: u32, window_ms: u32, hop_ms: u32) -> PyinProcessor {
    PyinProcessor::new(sample_rate_hz, window_ms, hop_ms)
}

pub fn push_and_get_midi(proc: &mut PyinProcessor, pcm16le_bytes: Vec<u8>) -> u16 {
    let run = || -> u16 {
        if proc.invalid_config || proc.pyin.is_none() {
            return UNVOICED_MIDI;
        }

        // Keep parsed mono samples in a deque to avoid O(n) front shifts while
        // advancing by hop size across calls.
        parse_pcm16le_bytes(proc, &pcm16le_bytes);

        if proc.sample_queue.len() < proc.frame_size_samples {
            return UNVOICED_MIDI;
        }

        let mut latest_midi = UNVOICED_MIDI;

        while proc.sample_queue.len() >= proc.frame_size_samples {
            for (idx, sample) in proc.sample_queue.iter().take(proc.frame_size_samples).enumerate() {
                proc.frame_buf[idx] = *sample;
            }

            // Feed exactly one hop worth of new data into the existing pYIN stream engine.
            let mut hop_bytes = Vec::with_capacity(proc.hop_size_samples * 2);
            for sample in proc.sample_queue.iter().take(proc.hop_size_samples) {
                hop_bytes.extend_from_slice(&sample.to_le_bytes());
            }

            if let Some(pyin) = proc.pyin.as_mut() {
                match pyin.push_bytes(&hop_bytes) {
                    Ok(frames) => {
                        for frame in frames {
                            if let Some(note) = frame.midi_note {
                                latest_midi = note as u16;
                            }
                        }
                    }
                    Err(err) => {
                        log::error!("pYIN push_bytes failed: {:?}", err);
                        return UNVOICED_MIDI;
                    }
                }
            }

            for _ in 0..proc.hop_size_samples.min(proc.sample_queue.len()) {
                proc.sample_queue.pop_front();
            }
        }

        latest_midi
    };

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)) {
        Ok(v) => v,
        Err(_) => {
            log::error!("push_and_get_midi panicked");
            UNVOICED_MIDI
        }
    }
}

fn parse_pcm16le_bytes(proc: &mut PyinProcessor, bytes: &[u8]) {
    let mut idx = 0usize;

    if let Some(prev) = proc.carry_byte.take() {
        if let Some(&next) = bytes.first() {
            proc.sample_queue.push_back(i16::from_le_bytes([prev, next]));
            idx = 1;
        } else {
            proc.carry_byte = Some(prev);
            return;
        }
    }

    while idx + 1 < bytes.len() {
        proc.sample_queue
            .push_back(i16::from_le_bytes([bytes[idx], bytes[idx + 1]]));
        idx += 2;
    }

    if idx < bytes.len() {
        proc.carry_byte = Some(bytes[idx]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odd_byte_leftover_across_pushes() {
        let mut proc = new_processor(48_000, 20, 10);
        let _ = push_and_get_midi(&mut proc, vec![0x34]);
        assert_eq!(proc.sample_queue.len(), 0);
        assert_eq!(proc.carry_byte, Some(0x34));

        let _ = push_and_get_midi(&mut proc, vec![0x12, 0x78, 0x56]);
        assert_eq!(proc.sample_queue.front().copied(), Some(0x1234));
        assert_eq!(proc.sample_queue.get(1).copied(), Some(0x5678));
    }

    #[test]
    fn midi_conversion_values_via_public_helper() {
        assert_eq!(crate::midi::midi_from_hz(220.0), 57);
        assert_eq!(crate::midi::midi_from_hz(440.0), 69);
        assert_eq!(crate::midi::midi_from_hz(1046.50), 84);
    }
}
