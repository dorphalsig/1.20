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

/// Compatibility wrapper that allocates a new Vec.
pub fn parse_pcm_bytes(bytes: &[u8], format: PcmFormat, leftover: &mut Vec<u8>) -> Vec<f32> {
    let mut out = Vec::new();
    parse_pcm_bytes_into(bytes, format, leftover, &mut out);
    out
}

/// Parse PCM bytes into normalized f32 samples in [-1, 1], appending to `out`.
pub fn parse_pcm_bytes_into(
    bytes: &[u8],
    format: PcmFormat,
    leftover: &mut Vec<u8>,
    out: &mut Vec<f32>,
) {
    match format {
        PcmFormat::I16LE => parse_i16le_into(bytes, leftover, out),
        PcmFormat::F32LE => parse_f32le_into(bytes, leftover, out),
    }
}

fn parse_i16le_into(bytes: &[u8], leftover: &mut Vec<u8>, out: &mut Vec<f32>) {
    let mut idx = 0;

    if !leftover.is_empty() {
        if let Some(&b1) = bytes.first() {
            let sample = i16::from_le_bytes([leftover[0], b1]);
            out.push(sample as f32 / 32768.0);
            leftover.clear();
            idx = 1;
        } else {
            return;
        }
    }

    let chunks = bytes[idx..].chunks_exact(2);
    let remainder = chunks.remainder();
    for pair in chunks {
        let sample = i16::from_le_bytes([pair[0], pair[1]]);
        out.push(sample as f32 / 32768.0);
    }

    leftover.clear();
    leftover.extend_from_slice(remainder);
}

fn parse_f32le_into(bytes: &[u8], leftover: &mut Vec<u8>, out: &mut Vec<f32>) {
    let mut idx = 0;

    if !leftover.is_empty() {
        let needed = 4 - leftover.len();
        if bytes.len() < needed {
            leftover.extend_from_slice(bytes);
            return;
        }

        let mut sample_bytes = [0_u8; 4];
        sample_bytes[..leftover.len()].copy_from_slice(leftover);
        sample_bytes[leftover.len()..4].copy_from_slice(&bytes[..needed]);
        let sample = f32::from_le_bytes(sample_bytes);
        out.push(sample.clamp(-1.0, 1.0));
        leftover.clear();
        idx = needed;
    }

    let chunks = bytes[idx..].chunks_exact(4);
    let remainder = chunks.remainder();
    for quad in chunks {
        let sample = f32::from_le_bytes([quad[0], quad[1], quad[2], quad[3]]);
        out.push(sample.clamp(-1.0, 1.0));
    }

    leftover.clear();
    leftover.extend_from_slice(remainder);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_parsing_roundtrip_i16le() {
        let samples: [i16; 4] = [0, 32767, -32768, 12345];
        let mut bytes = Vec::new();
        for s in samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }

        let mut leftover = Vec::new();
        let parsed = parse_pcm_bytes(&bytes, PcmFormat::I16LE, &mut leftover);
        assert!(leftover.is_empty());
        let decoded: Vec<i16> = parsed
            .iter()
            .map(|v| (v * 32768.0).round().clamp(-32768.0, 32767.0) as i16)
            .collect();
        assert_eq!(decoded, samples);
    }
}
