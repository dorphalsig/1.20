# F04 — Duet parsing: P1/P2 track routing

Covers Appendix F / F04.

## Contents

- `songs_root/a/valid_duet_interleaved/`:
  - Valid duet song with interleaved `P1`/`P2` markers.
  - Includes `expected.parsedSong.json` to assert track routing.
- `songs_root/b/invalid_duet_marker_p3/`:
  - Invalid duet marker (`P3`) must invalidate the song.

The fixture also provides `expected.discovery.json` to assert per-song validity and stable invalidation diagnostics.
