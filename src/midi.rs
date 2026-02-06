pub fn midi_from_hz(freq_hz: f32) -> u8 {
    if freq_hz <= 0.0 {
        return 0;
    }
    let midi = 69.0 + 12.0 * (freq_hz / 440.0).log2();
    midi.round().clamp(0.0, 127.0) as u8
}
