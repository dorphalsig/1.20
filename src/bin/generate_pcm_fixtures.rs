use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

fn sine_pcm(freq_hz: f32, seconds: f32, sample_rate: u32) -> Vec<u8> {
    let len = (seconds * sample_rate as f32) as usize;
    let mut out = Vec::with_capacity(len * 2);
    for i in 0..len {
        let t = i as f32 / sample_rate as f32;
        let v = (2.0 * std::f32::consts::PI * freq_hz * t).sin();
        let s = (v * 32767.0).clamp(-32768.0, 32767.0) as i16;
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn decode_ogg_to_mono_pcm16le(input: &Path) -> Vec<u8> {
    let file = File::open(input).expect("open ogg");
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("ogg");

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .expect("probe ogg");
    let mut format = probed.format;
    let track = format.default_track().expect("default track");
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .expect("make decoder");

    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut out = Vec::<u8>::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(e) => panic!("next_packet error: {e}"),
        };

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => panic!("decode error: {e}"),
        };

        let spec = *decoded.spec();
        let frames = decoded.frames();
        if sample_buf.as_ref().map_or(true, |b| b.capacity() < frames) {
            sample_buf = Some(SampleBuffer::<f32>::new(frames as u64, spec));
        }
        let buf = sample_buf.as_mut().unwrap();
        buf.copy_interleaved_ref(decoded);
        let ch = spec.channels.count();
        let s = buf.samples();

        if ch == 1 {
            for &v in s {
                let i = (v * 32767.0).clamp(-32768.0, 32767.0) as i16;
                out.extend_from_slice(&i.to_le_bytes());
            }
        } else {
            for frame in s.chunks_exact(ch) {
                let sum: f32 = frame.iter().copied().sum();
                let mono = sum / ch as f32;
                let i = (mono * 32767.0).clamp(-32768.0, 32767.0) as i16;
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
    }

    out
}

fn main() {
    let targets = [
        (
            "/mnt/data/sine-wave-a3-220hz.ogg",
            "a3_220_pcm16le_mono.pcm",
            220.0,
        ),
        (
            "/mnt/data/sine-wave-a4-440hz.ogg",
            "a4_440_pcm16le_mono.pcm",
            440.0,
        ),
        (
            "/mnt/data/sine-wave-c2-61,74hz.ogg",
            "c2_pcm16le_mono.pcm",
            65.41,
        ),
        (
            "/mnt/data/sine-wave-c6-1046,50hz.ogg",
            "c6_1046_50_pcm16le_mono.pcm",
            1046.50,
        ),
    ];

    let out_dir = Path::new("integration_test/assets/pcm");
    fs::create_dir_all(out_dir).expect("create output dir");

    for (input, output_name, fallback_hz) in targets {
        let pcm = if Path::new(input).exists() {
            decode_ogg_to_mono_pcm16le(Path::new(input))
        } else {
            eprintln!("warning: {} missing, generating synthetic {} Hz fallback", input, fallback_hz);
            sine_pcm(fallback_hz, 2.0, 48_000)
        };
        let out_path = out_dir.join(output_name);
        let mut f = File::create(&out_path).expect("create out pcm");
        f.write_all(&pcm).expect("write pcm");
        println!("wrote {} bytes to {}", pcm.len(), out_path.display());
    }
}
