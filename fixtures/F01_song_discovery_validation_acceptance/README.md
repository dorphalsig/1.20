# F01 — Song discovery + validation acceptance

## Purpose

Validates that the implementation:
- discovers UltraStar `.txt` files **recursively** under a configured root
- accepts valid songs and rejects invalid songs according to the spec validation rules
- emits invalidation diagnostics with stable `invalidReasonCode` and (when applicable) `invalidLineNumber`
- treats missing required audio as a fatal invalidation

## Root to scan

Scan this directory recursively:

- `fixtures/F01_song_discovery_validation_acceptance/songs_root`

## Expected outcomes

The deterministic fields that a test should assert are listed in:

- `fixtures/F01_song_discovery_validation_acceptance/expected.discovery.json`

Notes:
- `txtUri` values are relative to the fixture directory for portability.
- Dynamic fields like `songId`, `modifiedTimeMs`, or absolute SAF URIs are intentionally not asserted.
- Some cases include extra deterministic fields (`resolvedAudioRel`, `hasVideo`) to assert audio-resolution and missing-optional-asset behavior.

## Cases included

- `a/valid_minimal` — valid song with existing `audio.ogg`
- `a/invalid_missing_required_header` — missing required `#ARTIST` header (audio exists)
- `b/invalid_missing_audio` — `#AUDIO:missing.ogg` references a non-existent file
- `b/invalid_malformed_body` — body has a recognized note token with non-numeric duration

- `c/v1_audio_precedence` — VERSION>=1.0.0 with both `#AUDIO` and `#MP3`; expects `#AUDIO` wins (`resolvedAudioRel=audio.ogg`)
- `c/legacy_missing_mp3_invalid` — legacy (no VERSION) with `#AUDIO` but missing `#MP3`; expects invalid missing required header
- `c/legacy_mp3_preferred` — legacy with both `#AUDIO` and `#MP3`; expects `#MP3` chosen (`resolvedAudioRel=audio.mp3`)
- `c/v1_missing_optional_video` — VERSION>=1.0.0 with missing `#VIDEO` target; expects valid and treated as absent (`hasVideo=false`)
