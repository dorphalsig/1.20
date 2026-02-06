#[derive(Debug, Clone, Copy)]
pub enum PcmFormat {
    I16LE,
    F32LE,
}

impl Default for PcmFormat {
    fn default() -> Self {
        PcmFormat::I16LE
    }
}

/// Parse PCM bytes into normalized f32 samples in [-1, 1].
///
/// The parser keeps track of leftover bytes so that partial samples across
/// chunks are handled safely.
pub fn parse_pcm_bytes(bytes: &[u8], format: PcmFormat, leftover: &mut Vec<u8>) -> Vec<f32> {
    match format {
        PcmFormat::I16LE => parse_i16le(bytes, leftover),
        PcmFormat::F32LE => parse_f32le(bytes, leftover),
    }
}

fn parse_i16le(bytes: &[u8], leftover: &mut Vec<u8>) -> Vec<f32> {
    let mut data = Vec::with_capacity(leftover.len() + bytes.len());
    data.extend_from_slice(leftover);
    data.extend_from_slice(bytes);

    let mut samples = Vec::with_capacity(data.len() / 2);
    let mut idx = 0;
    while idx + 1 < data.len() {
        let sample = i16::from_le_bytes([data[idx], data[idx + 1]]);
        samples.push(sample as f32 / 32768.0);
        idx += 2;
    }

    leftover.clear();
    if idx < data.len() {
        leftover.push(data[idx]);
    }

    samples
}

fn parse_f32le(bytes: &[u8], leftover: &mut Vec<u8>) -> Vec<f32> {
    let mut data = Vec::with_capacity(leftover.len() + bytes.len());
    data.extend_from_slice(leftover);
    data.extend_from_slice(bytes);

    let mut samples = Vec::with_capacity(data.len() / 4);
    let mut idx = 0;
    while idx + 3 < data.len() {
        let sample = f32::from_le_bytes([data[idx], data[idx + 1], data[idx + 2], data[idx + 3]]);
        samples.push(sample.clamp(-1.0, 1.0));
        idx += 4;
    }

    leftover.clear();
    leftover.extend_from_slice(&data[idx..]);

    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_parsing_roundtrip_i16le() {
        let samples: [i16; 4] = [0, 32767, -32768, 12345];
        let mut bytes = Vec::new();
        for s in samples.iter() {
            bytes.extend_from_slice(&s.to_le_bytes());
        }

        let mut leftover = Vec::new();
        let parsed = parse_pcm_bytes(&bytes, PcmFormat::I16LE, &mut leftover);
        assert!(leftover.is_empty());
        let decoded: Vec<i16> = parsed
            .iter()
            .map(|v| (v * 32768.0).round() as i16)
            .collect();
        assert_eq!(decoded, samples);
    }
}
