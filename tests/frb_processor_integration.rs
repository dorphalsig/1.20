use pyin_rs::{new_processor, push_and_get_midi};
use std::fs;

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

#[test]
fn pcm_fixtures_expected_modes() {
    let fixtures = [
        ("integration_test/assets/pcm/a3_220_pcm16le_mono.pcm", 57),
        ("integration_test/assets/pcm/a4_440_pcm16le_mono.pcm", 69),
        ("integration_test/assets/pcm/c6_1046_50_pcm16le_mono.pcm", 84),
        ("integration_test/assets/pcm/c2_pcm16le_mono.pcm", 36),
    ];

    for (path, expected) in fixtures {
        let bytes = fs::read(path).expect("read pcm fixture");
        let (window_ms, hop_ms) = if path.contains("c6_") { (25, 5) } else { (43, 5) };
        let mut voiced = stream_collect(&bytes, 48_000, window_ms, hop_ms);
        assert!(voiced.len() >= 10, "{} had insufficient voiced outputs", path);
        voiced.drain(0..voiced.len().min(3));
        let m = mode(&voiced).expect("mode exists");
        assert_eq!(m, expected, "fixture {} mode was {}", path, m);
    }
}
