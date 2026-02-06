use pyin_rs::{new_processor, push_and_get_midi};
use std::fs;
use std::path::Path;

const CHUNK_PATTERN: [usize; 7] = [511, 1023, 2048, 333, 4097, 777, 1500];

fn stream_collect(bytes: &[u8], sample_rate_hz: u32, window_ms: u32, hop_ms: u32) -> Vec<u16> {
    let mut proc = new_processor(sample_rate_hz, window_ms, hop_ms);
    let mut mids = Vec::new();
    let mut offset = 0usize;
    let mut chunk_idx = 0usize;
    while offset < bytes.len() {
        let n = CHUNK_PATTERN[chunk_idx % CHUNK_PATTERN.len()].min(bytes.len() - offset);
        let chunk = bytes[offset..offset + n].to_vec();
        let midi = push_and_get_midi(&mut proc, chunk);
        if midi != 255 {
            mids.push(midi);
        }
        offset += n;
        chunk_idx += 1;
    }
    mids
}

fn mode(values: &[u16]) -> Option<u16> {
    let mut counts = std::collections::BTreeMap::<u16, usize>::new();
    for &v in values {
        *counts.entry(v).or_default() += 1;
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(v, _)| v)
}

fn ensure_fixture(path: &str, freq_hz: f32, sample_rate: u32) {
    let out_path = Path::new(path);
    if out_path.exists() {
        return;
    }
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).expect("create fixture dir");
    }
    let seconds = 2.0f32;
    let len = (seconds * sample_rate as f32) as usize;
    let mut bytes = Vec::with_capacity(len * 2);
    for i in 0..len {
        let t = i as f32 / sample_rate as f32;
        let v = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
        let s = (v * 32767.0).clamp(-32768.0, 32767.0) as i16;
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    fs::write(out_path, bytes).expect("write synthetic fixture");
}

#[test]
fn pcm_fixtures_expected_modes() {
    let fixtures = [
        ("integration_test/assets/pcm/a3_220_pcm16le_mono.pcm", 57, 220.0),
        ("integration_test/assets/pcm/a4_440_pcm16le_mono.pcm", 69, 440.0),
        (
            "integration_test/assets/pcm/c6_1046_50_pcm16le_mono.pcm",
            84,
            1046.50,
        ),
        ("integration_test/assets/pcm/c2_pcm16le_mono.pcm", 36, 65.41),
    ];

    for (path, expected, freq_hz) in fixtures {
        ensure_fixture(path, freq_hz, 48_000);
        let bytes = fs::read(path).expect("read pcm fixture");
        let (window_ms, hop_ms) = if path.contains("c6_") { (25, 5) } else { (43, 5) };
        let mut voiced = stream_collect(&bytes, 48_000, window_ms, hop_ms);
        assert!(voiced.len() >= 10, "{} had insufficient voiced outputs", path);
        voiced.drain(0..voiced.len().min(3));
        let m = mode(&voiced).expect("mode exists");
        assert_eq!(m, expected, "fixture {} mode was {}", path, m);
    }
}
