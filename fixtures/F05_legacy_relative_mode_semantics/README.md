# F05 — Legacy RELATIVE mode semantics (<1.0.0)

Covers Appendix F / F05.

## Scenario

A legacy song (`#VERSION` absent → treated as 0.3.0) that enables `#RELATIVE:YES` and exercises:

- per-track RELATIVE running offsets via `- <startBeat> <delta>` sentence lines
- BPM change behavior in relative mode: `B <startBeat> <bpm>` uses `Rel[0]` (track 0 offset), even if the BPM line appears while the active track is `P2`

## Files

- `song_relative_duet_bpm_rel0/song.txt`
- `song_relative_duet_bpm_rel0/expected.parsedSong.json`
- `song_relative_duet_bpm_rel0/audio.mp3` (empty stub)
