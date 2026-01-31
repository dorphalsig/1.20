Android Karaoke Game
USDX Parity MVP Functional Specification

Version: 1.11
Date: 2026-01-31
Owner: TBD

Status: Draft



# Change Record (rolling, last 4 hours, Europe/Berlin)

| Timestamp | Author | Changes |
| --- | --- | --- |
| 2026-01-31 10:38 CET | Assistant | Clarify join/roster placement: show compact QR+code join widget on Song List; make roster management canonical in Settings > Connect Phones; align Pairing UX section accordingly. |
| 2026-01-31 06:43 CET | Assistant | Add Appendix F fixture guide: repo layout + manifest usage + acceptance inventory F01–F15 with required subcases; bump version/date. |



# How to Use This Spec

This document defines the functional behavior required to implement a minimal Android karaoke game that behaves like UltraStar Deluxe (USDX) for the agreed MVP scope. It is designed to be sufficiently explicit for AI-driven implementation.

Conventions:

- TBD = decision or detail not yet specified.

- Paritiy-critical = must match USDX behavior for compatibility.

- Defaults are explicitly stated; if not, behavior is unspecified and must be decided.

# Table of Contents

- 1. Product Contract
  - 1.1 Locked Product Decisions
  - 1.2 Definition of Done
- 2. Architecture Overview
  - 2.1 Components
  - 2.2 Data Responsibilities
- 3. Songs and Library
  - 3.1 Storage Access
  - 3.2 Discovery and Validation Rules
  - 3.3 Index Fields (Functional)
  - 3.4 Song List (Landing Screen)
  - 3.5 Search (MVP)
- 4. USDX TXT Format Support
  - 4.1 Supported Note Tokens
  - 4.2 Supported Header Tags and Semantics
  - 4.3 Error Handling
- 5. Timing and Beat Model (Parity-Critical)
  - 5.1 Authoritative Beat Definitions
  - 5.2 Beat-Time Conversion
  - 5.3 START/END/NOTESGAP
- 6. Scoring (Parity-Critical)
  - 6.1 Scoring Overview
  - 6.2 Note Types
  - 6.2.1 ScoreFactor constants
  - 6.3 Player Level / Tolerance
  - 6.4 Octave Normalization
  - 6.5 Line Bonus
  - 6.6 Rounding and Display
- 7. Multiplayer, Pairing, and Session Lifecycle
  - 7.1 Session States
  - 7.2 Pairing UX (TV)
  - 7.3 Pairing UX (Phone)
  - 7.4 Disconnect/Reconnect
- 8. Network Protocol
  - 8.1 Transport
  - 8.2 Control Messages
  - 8.3 Pitch Stream Messages
  - 8.4 Versioning and Compatibility
- 9. Time Sync, Jitter Handling, and Auto Delay
  - 9.1 Defaults
  - 9.2 Auto Mic Delay Adjust (ON by default)
- 10. UI Screens and Flows
  - 10.1 Global navigation and input
  - 10.2 Song preview playback
  - 10.3 Assign Singers overlay (per-song)
  - 10.4 Settings Screen
  - 10.5 Singing Screen
  - 10.6 Results
- Appendix A: Supported Tags Reference
- Appendix B: Protocol Schemas
- Appendix C: Parsed Song Model (Normative)
- Appendix D: ParsedSongModel Fixture Serialization (Normative)
- Appendix E: Worked Examples (Normative for fixtures)
- Appendix F: Fixtures Guide and Acceptance Inventory

# 1. Product Contract

- Goal: USDX-like karaoke gameplay (parity for parsing, timing, duet, rap, scoring, results).

- Platforms: Android TV host app + 2 Android phone mic clients.

- Connectivity: same-subnet Wi-Fi only; offline operation.

- Players: 2.

- Out of scope: online song store, party modes, editors, esports-grade calibration.

## 1.1 Locked Product Decisions

- Default per-player level: Normal.

- Line bonus: ON.

- Duet: YES; swap duet parts: YES.

- Rap: YES (presence-based); Freestyle: no scoring.

- Video backgrounds: YES.

- Instrumental (full-song) via #INSTRUMENTAL: YES; instrumental gaps indicator: YES; instrumental.txt variant: NO.

- Songs loaded from USB/internal via SAF folder picker; persisted URI permissions.

## 1.2 Definition of Done

Parity MVP PASS requires all parity-critical behaviors in this spec to be met, plus functional pairing and play flows operating reliably on typical home Wi-Fi.

# 2. Architecture Overview

## 2.1 Components

- TV Host App: library, chart parsing, playback, timing/beat computation, scoring, UI, session management.

- Phone Mic Client: mic capture + DSP (pitch), computes toneValid thresholding, streams frames to TV.

## 2.2 Data Responsibilities

TV is authoritative for: song timeline, beats, scoring, rendering. Phones are authoritative for: mic capture and pitch extraction only.

# 3. Songs and Library

## 3.1 Storage Access

 SAF folder picker (ACTION_OPEN_DOCUMENT_TREE) for one or more song root folders; persisted read permission.

## 3.2 Discovery and Validation Rules

USDX scans for **all `.txt` files recursively** under configured song folders. Each `.txt` is treated as a distinct song entry, even if multiple `.txt` files exist in the same folder.

**Validation (song acceptance)**
A song entry is accepted into the library if and only if all of the following checks pass. If any check fails, the song entry MUST be rejected and a diagnostic MUST be emitted (see Section 4.3).

1) Required header tags present
- `#TITLE` and `#ARTIST` MUST be present and non-empty.
- `#BPM` MUST be present and parseable as a **non-zero** floating-point number (USDX accepts any non-zero value).
- A required audio reference tag MUST be present:
  - For `#VERSION >= 1.0.0`: at least one of `#AUDIO` or `#MP3` MUST be present and non-empty. If both are present, `#AUDIO` takes precedence (Section 4.2).
  - For legacy format (`#VERSION` absent or `< 1.0.0`): `#MP3` MUST be present and non-empty. `#AUDIO` (if present) MUST be ignored for audio resolution (USDX behavior).

2) Required audio file exists
- The audio reference resolved by Section 4.2 MUST resolve to an existing file when interpreted relative to the `.txt` directory (subpaths are allowed), unless the resolved value is an absolute URI supported by the platform (if absolute URIs are not supported in MVP, treat them as missing).

3) Notes section parses without fatal errors
- The notes/body section MUST be parsed according to Section 4.1 and Section 4.3.
- Unknown tokens and recoverable grammar issues MUST be handled per Section 4.3 (warn and continue).
- Any fatal numeric parse error for a recognized token MUST reject the song entry.

4) Each track has at least one non-empty sentence after cleanup
After body parsing completes, validation MUST ensure each parsed track (single track, or both tracks for duet) contains singable structure:

- The track MAY omit sentence delimiters (`-`). If the body contains note events but no `-`, the parser MUST still construct at least one sentence/line container (USDX behavior).
  - `ERROR_CORRUPT_SONG_NO_BREAKS` is **reserved** for the invariant failure where the implementation cannot construct any sentence/line container after parsing.

- Empty sentences MUST be removed. An "empty sentence" is a sentence/line with zero note events after parsing (i.e., no `:`, `*`, `F`, `R`, `G` notes).
  - This cleanup is performed before the "no notes" check.

- After removing empty sentences, the track MUST contain at least one remaining sentence/line.
  - If a track contains zero sentences after cleanup, reject with reason `ERROR_CORRUPT_SONG_NO_NOTES`.


**Missing files**
Audio/video/instrumental files are validated for existence at load time:
- Missing required audio file -> load fails.
- Missing optional video/instrumental -> logged; song can still load (but feature disabled).

**MVP parity requirements**
- Mirror the recursive `.txt` discovery behavior.
- Reject songs missing the required header fields or required audio file.
- Keep invalid song diagnostics (error line number + reason) for export/troubleshooting.

## 3.3 Index Fields (Functional)

The library index MUST store enough information to render Song Select and Search without re-parsing every TXT file on every app start.

Normative minimum index record (per song)
- Identity / storage
  - `songId`: stable identifier derived from the TXT document URI (e.g., normalized URI string hash). Must be stable across app restarts.
  - `txtUri`: persisted SAF URI for the TXT file (or absolute path in non-SAF environments).
  - `songsFolderUri`: the root library folder URI that produced this entry (to support multiple roots).
  - `modifiedTimeMs`: last-modified timestamp of the TXT file at indexing time.
- Validation
  - `isValid`: boolean.
  - `invalidReasonCode`: required if `isValid=false` (short stable string; see Section 4.3).
  - `invalidLineNumber`: required if `isValid=false` and the failure is associated with a specific TXT line (1-based).
- Display fields
  - `artist`, `title` (required by validation rules).
  - `album` (optional).
- Flags (derived from parse)
  - `isDuet` (true if song is duet).
  - `hasRap` (true if any `R` or `G` notes exist).
  - `hasVideo` (true if a video reference exists and the file is present).
  - `hasInstrumental` (true if `#INSTRUMENTAL` exists and the file is present).
- Preview/seek metadata
  - `startSec` (from `#START`, default 0.0).
  - `previewStartSec` (computed as: PREVIEWSTART else START else 0.0; see Section 3.4 and Section 10.2).

Implementations MAY store additional fields (e.g., genre, year, cover/background URIs, videoGapSec) but the above is the minimum required for MVP behavior.


## 3.4 Song List (Landing Screen)

**Purpose**
- Always the landing screen (even if library is empty).
- Displays songs sorted by **Artist -> Album -> Title**.
- MVP has **no song queue/playlist**; only one song is selected and played at a time.

**Header actions**
- **Settings** button: opens Settings screen.
- **Search** button: opens Search overlay (see Section 3.5).

**Pairing (on landing)**
- The landing screen MUST show a compact session join widget: QR code + short join code for the current session endpoint (Section 8.1).
- The landing screen MUST NOT show a connected-device roster.
- The join widget SHOULD be placed in the top-right area of the screen to avoid disrupting the song list layout.
- Device roster management (Rename/Kick/Forget) is available only in Settings -> Connect Phones (Section 10.4.1).

**Empty state**
- If no songs are indexed, show:
 - No songs found.
 - Hint: Open Settings -> Song Library to add a songs folder.

**Song row display**
- Minimum: Title, Artist, Album (if present).
- Icons/flags (if known from index): Duet, Rap, Video, Instrumental available.

**Selection behavior**
- OK on a song opens **Assign Singers** overlay (Section 10.3).

**Song preview**
- MVP: 10s audio preview starting at `#PREVIEWSTART` if present; otherwise start at `#START` if present, else start at 0.0 seconds (or optionally the first note).
(USDX editor uses PREVIEWSTART heavily; selection-screen preview behavior is theme-dependent, so we define MVP behavior here.)

**Wireframe (USDX-aligned, spec-limited interactions)**
```text
+--------------------------------------------------------------------------------+
| ● song selection                                      ultrastar (clone)        |
|   choose your song                                                     [  QR ] |
|                                                                  Code: ABCD    |
+--------------------------------------------------------------------------------+
|                                                                                |
|   [Cover - Prev]        [Cover - Selected]           [Cover - Next]            |
|                                                                                |
|                      +---------------------------+                              |
|                      |         ARTIST           |                              |
|                      |         Title            |                              |
|                      |                     6/86 |                              |
|                      +---------------------------+                              |
|                                                                                |
+--------------------------------------------------------------------------------+
| Hints:  OK=Select Song   Search=Filter   Settings=Config   Back=Exit            |
+--------------------------------------------------------------------------------+
```

## 3.5 Search (MVP)

**User-visible behavior**
- Song list includes a **Search** action (button or icon in the header). Selecting it opens a Search overlay.
- Search overlay contains:
 - `Query` text field
 - `Scope` selector: `Everywhere` (default), `Artist`, `Album`, `Song`
 - Results list that updates as the query changes
- Matching is **case-insensitive substring** match.
 - `Artist` scope matches only the artist field.
 - `Album` scope matches only the album field.
 - `Song` scope matches only the title field.
 - `Everywhere` matches if any of {artist, album, title} match.
- Selecting a result behaves exactly like selecting that song in the main list (i.e., proceeds to Assign Singers overlay, Section 10.3).

**Focus and keyboard (normative)**
- On opening Search, focus MUST start on the Query field and the software keyboard MUST open.
- DPAD down from the keyboard focuses the Scope selector; DPAD down again focuses the Results list.
- The Query field MUST provide a Clear action to erase the current query.

**Wireframe (spec interactions; USDX-style modal)**
```text
+--------------------------------------------------------------------------------+
| SEARCH                                                                          |
+--------------------------------------------------------------------------------+
| Query: [ psy____________________ ]     [Clear]                                 |
| Scope:  (• Everywhere) (  Artist) (  Album) (  Song)                           |
+--------------------------------------------------------------------------------+
| Results (max 50; ordered like main list)                                       |
|  > PSY — Gangnam Style                                                         |
|    PSY — Gentleman                                                             |
|    ...                                                                         |
+--------------------------------------------------------------------------------+
| Hints: OK=Select Song   Back=Close                                              |
+--------------------------------------------------------------------------------+
```

**Result ordering (normative)**
- Search results MUST preserve the same ordering as the main Song List (Artist -> Album -> Title), filtered by the current query.

**Performance and memory constraints (normative for MVP)**
- Live filtering MUST be implemented as **O(N)** scan over the in-memory song index, where `N` is the number of songs.
- Input MUST be **debounced** by 150 ms.
- UI MUST cap displayed results to **50** (or fewer) to avoid render stalls.
- Store pre-normalized lowercase strings per song (`artistL`, `albumL`, `titleL`) to avoid repeated allocations during filtering.
- Optional: for `Everywhere`, implementations MAY precompute `allL = artistL + " " + albumL + " " + titleL` per song to reduce per-keystroke checks; this is not required.

# 4. USDX TXT Format Support

## 4.1 Supported Note Tokens

### Note/body line tokens (USDX parser)

USDX reads the song body line-by-line and interprets the first character token.

Supported tokens:
- `:` Normal note
- `*` Golden note
- `F` Freestyle note (scored as 0)
- `R` Rap note
- `G` RapGolden note
- `-` Line break / new sentence
- `E` End of song data
- `B` BPM change event inside song data
- `P1`, `P2` Duet part delimiters (body markers; must appear on their own line, starting with `P`)

### Per-note fields
For note tokens (`:`, `*`, `F`, `R`, `G`) USDX parses:
`<token> <startBeat> <duration> <tone> <lyricText...>`
- `startBeat` and `duration` are integers in chart beat units. They are not scaled by BPM; BPM affects only the beat->time conversion (Section 5.1). Any legacy relative-mode shift (format < 1.0.0) is applied separately (Section 4.2).
- `tone` is an integer note tone as stored in the file.
- `lyricText` is the remainder of the line after the numeric fields.

### Duet structure
- If the first non-empty body line begins with `P`, USDX marks the song as duet (`isDuet = true`) and creates two tracks.
- A `P1`/`P2` marker sets the active track (0/1).
- Notes and `-` sentence breaks are assigned to the current active track.
- The file ends with a single `E` after all notes.

## 4.2 Supported Header Tags and Semantics

### Required tags
- `#TITLE:` song title (UTF-8 for format >= 1.0.0).
- `#ARTIST:` song artist.
- `#BPM:` base BPM. USDX loads as `BPM_internal = BPM_file * 4`.
- Audio filename:
 - For `#VERSION >= 1.0.0`: at least one of `#AUDIO:` or `#MP3:` MUST be present and non-empty. If `#AUDIO:` is present, it takes precedence over `#MP3:` (USDX behavior).
 - For legacy format (`#VERSION` absent or `< 1.0.0`): `#MP3:` MUST be present and non-empty. `#AUDIO:` (if present) MUST be ignored for audio resolution (USDX behavior).
 - The resolved audio file MUST exist, otherwise load fails.

### Timing/alignment tags
- `#GAP:` millisecond offset used as the lyrics/audio time origin for beat/time conversions (see Section 5.1). Parsed as a float (fractional ms allowed).
- `#START:` seconds; initial playback/lyrics time offset.
- `#END:` milliseconds; sets lyrics total time if present.
- `#PREVIEWSTART:` seconds; used by editor and can be used for song preview.

### Media tags
- `#VIDEO:` video filename.
- `#VIDEOGAP:` seconds offset added to audio position when positioning video.
- `#INSTRUMENTAL:` alternate audio file used for instrumental/karaoke mode.
- `#COVER:` image; `#BACKGROUND:` image; with fallbacks `*[CO].jpg` and `*[BG].jpg` if unset.

### Duet tags
Singer labels (selection/menu only; not the duet body delimiter):
- `#P1:` and `#P2:` set duet singer names.
- Legacy `#DUETSINGERP1:` / `#DUETSINGERP2:` are only honored for format <1.0.0; ignored for 1.0.0.

### Legacy/deprecated tags
- `#ENCODING:` ignored for format >= 1.0.0 (UTF-8 is forced); honored for older formats.
- `#RESOLUTION:` and `#NOTESGAP:` honored only for format <1.0.0; ignored otherwise.
- `#RELATIVE:` honored only for format <1.0.0; for 1.0.0 the song is rejected (not loaded).

### In-song BPM changes
- Body lines starting with `B` define variable BPM segments: `B <startBeat> <bpm>`.

## 4.3 Error Handling

**Implementation requirements (MVP, parity-aligned)**

**Header tags**

Header processing is best-effort and MUST continue past unknown or non-fatal issues.

- Header lines are read from the top of the file while the first character of the line is `#`. Any other line (including an empty line) ends header parsing (USDX behavior).
- Tag names are case-insensitive; matching MUST be performed on `Uppercase(Trim(TagName))`.
- Duplicate known tags: if the same known tag appears multiple times, the **last** successfully parsed value wins (earlier values are overwritten).
- Each header line is classified into exactly one of:
  - **Well-formed tag**: `#NAME:VALUE` where `NAME` is non-empty.
  - **No separator**: a line starting with `#` that contains no `:`.
  - **Empty value**: `#NAME:` (value is empty string after trimming).

For each header line:
- **Well-formed known tag**: parse according to its definition.
  - If the value is malformed:
    - If the tag is **required for validity** (TITLE/ARTIST/AUDIO-or-MP3/BPM): mark the song **invalid**.
    - If the tag is **optional** (VIDEO, COVER, BACKGROUND, INSTRUMENTAL, etc.): **warn** and treat as absent.
- **Well-formed unknown tag**: **warn** and preserve it in `CustomTags` as `(NAME, VALUE)`.
- **Empty value** (`#NAME:`): **info/warn** and preserve it in `CustomTags` as `(NAME, "")`.
- **No separator** (no `:`): **warn** and preserve it in `CustomTags` as `("", CONTENT)` where `CONTENT` is the original line without the leading `#`.

`CustomTags` representation (MVP):
- `CustomTags` is an ordered list of `(TagName, Content)` pairs.
- `TagName` may be empty only for the "no separator" case above.
- The stored strings MUST be exactly the trimmed forms described above (do not reformat).

**Media files**
- Missing/unresolvable required audio file: **invalid**.
- Missing optional assets (video/images/instrumental): **warn** and continue without that asset.
- If video fails to open/decode at runtime: fall back to background/visualization without interrupting scoring/playback.

**Body grammar (notes section)**

Body parsing is best-effort and MUST continue past unknown or non-fatal issues. The goal is to load as much as possible while preserving deterministic behavior.

Recognized leading tokens (first non-whitespace character of the line):
- `E` end of song
- `P` duet track marker (`P1` or `P2`)
- Note tokens: `:` normal, `*` golden, `F` freestyle, `R` rap, `G` rap-golden
- Sentence marker: `-`
- BPM change marker: `B`

Rules:
- If the token is unrecognized: **warn** with line number and ignore the line.
- If the token is recognized but required numeric fields cannot be parsed as integers/floats: **invalid** (fatal for that song).

Token-specific behavior:
- `E`: stop reading the body; the song load continues with validation.
- `P`:
  - Accept only `P1` or `P2` (after the `P`).
  - Any other `P` value: **invalid** (fatal for that song).
- Note tokens (`:`, `*`, `F`, `R`, `G`):
  - Parse required fields as integers:
    - `startBeat` (int)
    - `duration` (int)
    - `tone` (int)
    - `lyricText` is the remainder of the line (may be empty).
  - Auto-fix: if `duration == 0`, then:
    - **warn** with line number: "found note with length zero -> converted to FreeStyle"
    - convert the note token to `F` (freestyle) and keep `duration` unchanged (still zero).
  - Optional conversion flags (MVP settings-controlled):
    - If `RapToFreestyle == true` and token is `R`, store it as freestyle instead of rap.
    - If `OutOfBoundsToFreestyle == true` and the note is before audio start or after audio end (as defined by the timing model), convert it to `F` and warn.
- `-` (sentence): parse required integer `startBeat` (and, if the song is in "relative" mode, also parse the second integer parameter). If parsing fails: **invalid**.
- `B` (BPM change): parse required floats `startBeat` and `bpm`. If parsing fails: **invalid**.

**Version/encoding**
- If `#VERSION` is absent, treat the song as legacy format `0.3.0`.
- If `#VERSION` is present, it MUST parse as a dotted numeric version (e.g., `1.0.0`). If it fails to parse: **invalid**.
- Supported versions are `< 2.0.0`. If `#VERSION >= 2.0.0`: **invalid**.
- For `VERSION >= 1.0.0`, treat file as UTF-8; ignore `#ENCODING` with a warn/info log.
- For legacy versions, apply `#ENCODING` if present; decode failure -> invalid.

**Logging**

All invalidation MUST include a concise reason string suitable for display in a debug invalid songs listing.

Diagnostics record schema (normative)
- Implementations MUST produce a structured diagnostics list per song load attempt, where each entry has:
  - `severity`: one of `info` | `warn` | `invalid`.
  - `code`: short stable string (see the minimum code set below).
  - `message`: human-readable description.
  - `txtUri`: song TXT identifier.
  - `lineNumber`: optional 1-based line number within the TXT file, present whenever a specific line caused the issue.
- For any `isValid=false` song (Section 3.3), there MUST be at least one diagnostics entry with `severity=invalid`, and the song's `invalidReasonCode` MUST equal that entry's `code`.

Minimum invalidation codes (parity-aligned)
- `ERROR_CORRUPT_SONG_FILE_NOT_FOUND`: required audio file missing/unresolvable.
- `ERROR_CORRUPT_SONG_NO_NOTES`: after sentence cleanup, no remaining sentences.
- `ERROR_CORRUPT_SONG_NO_BREAKS`: reserved — implementation could not construct any sentence/line container after parsing (USDX typically does not reject songs solely for missing `-`).
- `ERROR_CORRUPT_SONG_MISSING_REQUIRED_HEADER`: missing TITLE/ARTIST/AUDIO-or-MP3/BPM.
- `ERROR_CORRUPT_SONG_MALFORMED_HEADER`: required header present but malformed/unparseable.
- `ERROR_CORRUPT_SONG_MALFORMED_BODY`: recognized body token but numeric field parse fails.
- `ERROR_CORRUPT_SONG_INVALID_VERSION`: VERSION exists but fails to parse, or VERSION >= 2.0.0.
- `ERROR_CORRUPT_SONG_INVALID_DUET_MARKER`: `P` token present with value other than P1/P2.

### Legacy RELATIVE semantics (parity-critical for format < 1.0.0)

If `#RELATIVE:YES` is present (and `#VERSION` is absent or `< 1.0.0`), the file uses a running beat-offset per track.

State:
- `Rel[track]` starts at 0 for each track (P1/P2).

Rules:
- For a sentence line `- <startBeat> <delta>` (two integers in relative mode):
  - `effectiveStartBeat = startBeat + Rel[track]`
  - Set the new line start beat to `effectiveStartBeat`.
  - Then update `Rel[track] = Rel[track] + delta`.
- For a note line `<token> <startBeat> <duration> <tone> ...` in relative mode:
  - `effectiveStartBeat = startBeat + Rel[track]`.
  - `duration` is not affected by `Rel`.
- For an in-song BPM line `B <startBeat> <bpm>` in relative mode:
  - `effectiveStartBeat = startBeat + Rel[0]` (track 0 offset is used, even in duet).

If `#RELATIVE` is present with a value other than `YES`, treat it as `NO`.

# 5. Timing and Beat Model (Parity-Critical)

## 5.1 Authoritative Beat Definitions

The chart is authored in beats, while DSP frames and playback run in time. The TV MUST convert between time and beats deterministically using the rules below.

Definitions:
- `GAPms`: the float value of `#GAP:` in milliseconds (fractional ms allowed).
- `lyricsTimeSec`: the current lyrics/playback clock time in seconds, where `lyricsTimeSec = 0` corresponds to the start of the audio file.
- `micDelayMs`: the per-phone (or per-player) microphone delay setting in milliseconds.

Two beat cursors are used:

1) Highlight beat cursor (UI timing)
- `highlightTimeSec = lyricsTimeSec - (GAPms / 1000.0)`
- `CurrentBeat = floor(TimeSecToMidBeatInternal(highlightTimeSec))`

2) Scoring beat cursor (judgement timing)
- `scoringTimeSec = lyricsTimeSec - ((GAPms + micDelayMs) / 1000.0)`
- `CurrentBeatD = floor(TimeSecToMidBeatInternal(scoringTimeSec) - 0.5)`

Notes:
- `floor()` MUST be mathematical floor.
- The `- 0.5` in `CurrentBeatD` is required to match USDX timing: it shifts scoring decisions half a beat earlier.

## 5.2 Beat-Time Conversion

### Internal beat unit

USDX treats the beat numbers written in UltraStar `.txt` files as the authoritative beat grid (quarter-beat resolution). There is no additional beat scaling.

- File beats: the integers stored in note lines (`startBeat`, `duration`) and sentence lines (`- startBeat`) in the `.txt`.
- Internal beats: identical to file beats (no scaling): `internalBeat = fileBeat`.

Parsing rule:
- Parsed beat values (note `startBeat`, note `duration`, sentence `startBeat`, and BPM-change `startBeat`) MUST be used as-is (no `*4`).

### Internal BPM

- The `.txt` header `#BPM:` is expressed in file beats per minute.
- The internal BPM is:
  - `BPM_internal = BPM_file * 4`

For BPM changes inside the song body (`B <startBeat> <bpm>`):
- Parse `startBeat_file` and `bpm_file`.
- Convert:
  - `startBeat_internal = startBeat_file` (no scaling)
  - `bpm_internal = bpm_file * 4`

### TimeSecToMidBeatInternal

`TimeSecToMidBeatInternal(tSec)` converts a time offset (seconds) into an internal beat position (float).

Input:
- `tSec` is measured relative to the chart origin (i.e., `lyricsTimeSec - GAPms/1000.0`), and MAY be negative.

Output:
- A floating-point internal beat position.

Static BPM (no `B` lines):
- `MidBeat_internal = tSec * (BPM_internal / 60.0)`

Variable BPM (one or more `B` lines):
- Let `segments` be the BPM segment list in ascending `startBeat_internal`, starting with segment 0 at `startBeat_internal = 0` with `bpm_internal = header_BPM_internal`.
- For each segment `i` with `(startBeat_i, bpm_i)` and next segment start `startBeat_{i+1}` (or infinity for the last segment), define:
  - `segBeats = startBeat_{i+1} - startBeat_i` (for the last segment, treat `segBeats = +infinity`)
  - `secPerBeat = 60.0 / bpm_i`
  - `segTime = segBeats * secPerBeat`
- Conversion algorithm:
  - If `tSec <= 0`, return `MidBeat_internal = 0` (clamp; USDX behavior for variable BPM).
  - Else, walk segments from i=0 upward:
    - If `tSec >= segTime`, then `tSec -= segTime` and add `segBeats` to the beat accumulator.
    - Else, add `tSec * (bpm_i / 60.0)` to the beat accumulator and stop.

### BeatInternalToTimeSec

`BeatInternalToTimeSec(beatInt)` converts an internal beat index to a time offset in seconds, relative to the chart origin (i.e., `lyricsTimeSec - GAPms/1000.0`).

Static BPM (no `B` lines):
- `tSec = beatInt * (60.0 / BPM_internal)`

Variable BPM:
- Using the same segment definition as above, walk segments:
  - Initialize `tSec = 0`.
  - For each segment `i`:
    - If `beatInt >= startBeat_{i+1}` (i.e., the beat lies after the segment end), add full segment time: `(startBeat_{i+1} - startBeat_i) * (60.0 / bpm_i)`.
    - Else, add remaining time in this segment: `(beatInt - startBeat_i) * (60.0 / bpm_i)` and stop.


To convert this chart-relative time back to `lyricsTimeSec` (audio-start relative), add `GAPms/1000.0`.
Boundary conventions:
- When comparing a time to a note window converted from beats, implementations MUST use: `noteActive if startBeat <= beat < endBeat` (start inclusive, end exclusive).

## 5.3 START/END/NOTESGAP

This section defines how the optional TXT headers `#START` and `#END` affect playback, and how legacy `#NOTESGAP`/`#RESOLUTION` behave.

START (normative)
- `#START:` is parsed as a float seconds value `startSec`.
- When entering the Singing screen in normal play, the song timeline `songTimeSec` and audio playback position MUST be initialized to `startSec`.
- If a video is present, its playback position MUST be initialized to `videoGapSec + startSec` (see Section 4.2 for `videoGapSec`).

END (normative)
- `#END:` is parsed as an integer milliseconds value `endMs`.
- If `endMs > 0`, the song MUST end when `songTimeSec >= endMs/1000.0` (after applying the same start initialization above).
- If `endMs <= 0` or missing, the song duration is determined by the audio track length.

NOTESGAP and RESOLUTION (normative)
- These headers are honored only for format versions < 1.0.0. For format version >= 1.0.0 they MUST be ignored with an info log.
- When honored, NOTESGAP/RESOLUTION affect only beat-click scheduling and editor/drawing beat delimiter alignment. They MUST NOT affect scoring.

Gameplay behaviors that depend on START/END (normative)
- Restart song: resets per-player scores/state and seeks playback back to `startSec` (and video to `videoGapSec + startSec`).
- Skip intro action: when triggered during singing, if the next upcoming line start time is more than 6.0 seconds ahead of current `songTimeSec`, seek to 5.0 seconds before that next line start time.
  - Definition (parity-critical): the "next upcoming line start time" MUST be computed from the lyrics renderer's *next-to-sing* ("upper") line, using the start beat of the *first note* in that line.
    - Solo: `nextLineStartBeat = UpperLine.StartNote`.
    - Duet: `nextLineStartBeat = min(UpperLineP1.StartNote, UpperLineP2.StartNote)`.
    - Convert to seconds as: `nextLineStartTimeSec = BeatInternalToTimeSec(nextLineStartBeat) + (GAPms/1000.0)`.
  - The seek target MUST be clamped to at least `startSec`.



# 6. Scoring (Parity-Critical)

## 6.1 Scoring Overview

 Beat-based scoring, normalized to 10000 total. Line bonus ON reserves 1000 for line bonus and distributes remaining points via note value normalization.

Scoring beat stepping (parity-critical)
- Implementations MUST evaluate scoring on every integer detection beat `b` in the interval `(oldBeatD, currentBeatD]` (i.e., from `oldBeatD+1` through `currentBeatD`, inclusive).
  - `oldBeatD` and `currentBeatD` are the scoring-beat cursors defined in Section 5.1 (CurrentBeatD is derived from scoring time and includes the `-0.5` offset before flooring).
- For each evaluated beat `b`, the active note window test MUST use the boundary convention from Section 5.2: `noteActive if startBeat <= b < endBeat`.

## 6.2 Note Types

Note-type tokens in the TXT file:
- Freestyle: `F`
- Normal: `:`
- Golden: `*`
- Rap: `R`
- RapGolden: `G`

Per-detection-beat scoring eligibility:
- Freestyle notes (`F`) are excluded from hit detection and contribute 0 points.
- For Normal (`:`) and Golden (`*`) notes: a detection beat can score only if `toneValid=true` and pitch is within the tolerance Range after octave normalization (Sections 6.3-6.4).
- For Rap (`R`) and RapGolden (`G`) notes: a detection beat can score if `toneValid=true`; pitch difference is ignored (presence-only).

Definition of `toneValid` and how it is produced/transported is normative in Section 8.3 (Pitch Stream Messages).

## 6.2.1 ScoreFactor constants

ScoreFactor is used to weight note durations for score normalization and line bonus calculations.

Normative constants:
- Freestyle (`F`): ScoreFactor=0
- Normal (`:`): ScoreFactor=1
- Golden (`*`): ScoreFactor=2
- Rap (`R`): ScoreFactor=1
- RapGolden (`G`): ScoreFactor=2

## 6.3 Player Level / Tolerance

Each singer/player has a Difficulty setting: Easy, Medium, or Hard.

Define the pitch tolerance Range (in semitones) as:
- Easy: Range = 2
- Medium: Range = 1
- Hard: Range = 0

Range is applied only for Normal and Golden notes (Section 6.2). Rap notes ignore pitch difference.

Default Difficulty is Medium for each newly assigned singer.

**Parity requirement**
Implement the exact Range mapping above, per player.

## 6.4 Octave Normalization

Before comparing to the target note, USDX normalizes the detected pitch **to the closest octave of the target note**, but it does so using the detected pitch-class (`Tone`) and shifting it by 12:

```
while (Tone - TargetTone > 6) Tone := Tone - 12
while (Tone - TargetTone < -6) Tone := Tone + 12
```



**Notes**
- Phones send `midiNote` (integer semitone index, MIDI note number). The TV derives the USDX-compatible semitone scale:
  - `toneUsdx = midiNote - 36` (so C2=36 maps to `toneUsdx=0`, matching USDX's C2=0 pitch base)
  - `Tone = toneUsdx mod 12` (pitch class)
- After octave normalization, the value compared/scored is the normalized `Tone` (potentially outside 0..11).

**Parity requirement**
Implement octave normalization exactly as above (shift detected `tone` by 12 until it is within 6 semitones of the target note).

## 6.5 Line Bonus

Line bonus is a scoring mode that reserves 1000 points of the 10000-point total for sentence/line completion.

Enable/disable (normative):
- Setting `LineBonusEnabled` (boolean), default ON.
- If OFF: `MaxSongPoints = 10000` and `MaxLineBonusPool = 0`.
- If ON: `MaxSongPoints = 9000` (notes+golden budget) and `MaxLineBonusPool = 1000`.

Per-line max score (normative):
- Each track computes `TrackScoreValue = sum(Note.Duration * ScoreFactor[noteType])` over all notes in the track (Section 6.2.1).
- Each line/sentence computes `LineScoreValue = sum(Note.Duration * ScoreFactor[noteType])` over its notes.
- For a line, define the note-score budget available to that line as:
  `MaxLineScore = MaxSongPoints * (LineScoreValue / TrackScoreValue)`

Line perfection (normative):
At sentence end:
- `LineScore = (Player.Score + Player.ScoreGolden) - Player.ScoreLast`
- If `MaxLineScore <= 2` then `LinePerfection = 1`
- Else `LinePerfection = clamp(LineScore / (MaxLineScore - 2), 0, 1)`

Line bonus distribution (normative, when LineBonusEnabled=ON):
- A line is empty if `LineScoreValue = 0`. Empty lines do not receive line bonus.
- Let `NonEmptyLines = NumLines - NumEmptyLines`. Then:
  - `LineBonusPerLine = MaxLineBonusPool / NonEmptyLines`
  - `Player.ScoreLine += LineBonusPerLine * LinePerfection`

Rounding: see Section 6.6.

**Parity requirement**
Implement sentence-end scoring and line bonus exactly as above, including the `-2` forgiveness term.

## 6.6 Rounding and Display

Per-beat note scoring (normative):
- Let `MaxSongPoints` be as defined in Section 6.5 (10000 if LineBonusEnabled=OFF; 9000 if ON).
- Let `TrackScoreValue` be as defined in Section 6.5.
- For each detection beat where the active note is considered hit (Section 6.2):
  - `CurBeatPoints = (MaxSongPoints / TrackScoreValue) * ScoreFactor[noteType]`
  - If noteType is Normal or Rap: add to `Player.Score`
  - If noteType is Golden or RapGolden: add to `Player.ScoreGolden`

Line score rounding (normative):
- `Player.ScoreLineInt = floor(round(Player.ScoreLine) / 10) * 10`

Tens rounding (normative):
- `ScoreInt = round(Player.Score/10) * 10`
- `ScoreGoldenInt` is rounded to tens in the opposite direction to ensure the sum cannot exceed 10000 due to .5 rounding:
  - If `ScoreInt < Player.Score` then `ScoreGoldenInt = ceil(Player.ScoreGolden/10) * 10`
  - Else `ScoreGoldenInt = floor(Player.ScoreGolden/10) * 10`
- `ScoreTotalInt = ScoreInt + ScoreGoldenInt + Player.ScoreLineInt`

Parity requirement:
Use the exact rounding rules above and compute total as shown.

# 7. Multiplayer, Pairing, and Session Lifecycle

## 7.1 Session States

Session state is owned by the TV host app.

**States (normative)**
- **Open**: phones may join and appear in the connected-roster.
- **Locked**: a song is in progress; new joins are rejected (existing phones may reconnect).
- **Ended**: the current session token is invalid; all phones must join a new session.

**Lifecycle (normative)**
- On TV app launch, the host MUST create a new session in state **Open** and display pairing info.
- When Singing starts, the session enters **Locked**.
- When the user returns to Song List after song end/quit, the session returns to **Open**.
- The session enters **Ended** only when the host explicitly ends it via Settings > Connect Phones (**End session**) or when the app is closed.

**Pairing across sessions (normative for MVP)**
- Reconnect-within-session is supported (Section 7.4).
- Persistent singer assignment across sessions is NOT supported: on a new session, all phones join as spectators until assigned for a song (Section 10.3).

## 7.2 Pairing UX (TV)

**Join UI placement (normative)**
- The TV host MUST display the session join QR code and short join code representing the current session endpoint (Section 8.1).
- The Song List landing screen (Section 3.4) MUST show a compact join widget (QR + code) and MUST NOT show the connected-device roster.
- Settings -> Connect Phones (Section 10.4.1) MUST show the join QR/code plus the connected-device roster and management actions.

**Join admission (normative)**
- Phones MAY join while the session is **Open** until the roster reaches 10 devices.
- Additional phones MUST be rejected with an `error` (e.g., `code="session_full"`).
- During **Locked** state, new joins MUST be rejected with an `error` (e.g., `code="session_locked"`).

**Roster actions (normative)**
- **Rename device**: changes the display label shown on TV and stored by `clientId` for future use within the same session.
- **Kick device**: disconnects the device immediately; the roster entry is removed.
- **Forget device**: removes the stored display label for that `clientId` and disconnects the device; a future join is treated as a fresh device with default name.
- Kick/Forget MUST use a confirm dialog with default focus on Cancel.

**Wireframes**
- Join widget: see Section 3.4 (Song List).
- Roster management: see Section 10.4.1 (Settings > Connect Phones).

## 7.3 Pairing UX (Phone)

- Phone joins by scanning the TV QR code or entering the join code.
- Phone shows:
 - Connection state (Connecting / Connected / Disconnected)
 - Current assigned role (Singer / Spectator); if Singer, show playerId (P1/P2)
 - Live input level meter
 - Mute toggle: when enabled, the phone MUST continue to stay connected but MUST stream frames as unvoiced (equivalent to `toneValid=false` and no `midiNote`) so the TV scores silence.
 - Leave session action

**Wireframes (phone app, spec-only interactions)**
```text
Join screen

+----------------------------------+
| JOIN SESSION                      |
+----------------------------------+
| [Scan QR]                         |
| or enter code: [ ABCD ] [Join]    |
|                                  |
| Status: Disconnected              |
+----------------------------------+

Connected (Spectator)

+----------------------------------+
| CONNECTED                         |
+----------------------------------+
| Role: Spectator                   |
| Input level:  |||||||             |
| Mute: [OFF] (streams silence when ON)
|                                  |
| [Leave session]                   |
+----------------------------------+

Assigned as Singer (during a song)

+----------------------------------+
| CONNECTED                         |
+----------------------------------+
| Role: Singer (P1)                 |
| Status: Waiting / Singing         |
| Input level:  |||||||||           |
| Mute: [OFF]                       |
|                                  |
| [Leave session]                   |
+----------------------------------+
```

**Join rejection UX (normative)**
- If the TV rejects a join with an `error`, the phone MUST show a blocking error message and return to the Join screen.
- Minimum user action is `OK` (dismiss) or Back.

**Wireframes (phone join rejected; spec-only interactions)**
```text
Session locked

+----------------------------------+
| ERROR                             |
+----------------------------------+
| Session is locked.                |
| (A song is in progress.)          |
|                                  |
| [OK]                              |
+----------------------------------+

Session full

+----------------------------------+
| ERROR                             |
+----------------------------------+
| Session is full.                  |
|                                  |
| [OK]                              |
+----------------------------------+

Protocol mismatch

+----------------------------------+
| ERROR                             |
+----------------------------------+
| Protocol mismatch.                |
|                                  |
| [OK]                              |
+----------------------------------+
```

## 7.4 Disconnect/Reconnect

- Gameplay does not pause on disconnect.
- While disconnected, that player contributes no pitch frames and MUST receive no additional score.
- If the same phone reconnects within the same session, it SHOULD reclaim its prior identity using `clientId` (Section 8.2 `hello`).
- If the phone was assigned as a Singer when it disconnected, it MUST resume that role on reconnect (unless the host has cleared assignments).
- If the session roster is full and the reconnect cannot be matched to an existing `clientId`, the reconnect MUST be rejected with `code="session_full"`.

# 8. Network Protocol

## 8.1 Transport

**Implementation requirements (MVP)**

- Transport MUST be **WebSocket** over the local network (same subnet WiFi).
- TV host exposes: `ws://<host-ip>:<port>/?token=<sessionToken>`.
- **Session token**
 - Cryptographically random string, minimum 128 bits entropy (e.g., 22+ chars base64url).
 - Generated per Session start; invalidated when Session ends.
 - Reuse across sessions is NOT allowed.
- Host MUST reject connections with missing/incorrect token and send an `error` before closing.

## 8.2 Control Messages

**Implementation requirements (MVP)**

All messages are JSON objects with fields:
- `type` (string), `protocolVersion` (int), `tsTvMs` (optional; TV may include).

Required control messages:

1) `hello` (phone -> TV) 
- Fields: `clientId` (stable UUID), `deviceName`, `appVersion`, `protocolVersion`, `capabilities` (e.g., `{"pitchFps":50}`)

2) `assignPlayer` (TV -> phone) 
- Fields: `playerId` (`"P1"` or `"P2"`), `thresholdIndex` (0..7), `effectiveMicDelayMs` (optional for display/debug)

3) `sessionState` (TV -> phone, and optional phone -> TV ack) 
- Fields: `sessionId`, `slots` (`{"P1":{connected,deviceName}, "P2":{...}}`), `inSong` (bool), `songTimeSec` (float, optional)

4) `ping` / `pong` (both directions) 
- `ping` fields: `nonce`, `tTvSendMs` (TV time) or `tPhoneSendMs` (phone time) depending on sender 
- `pong` echoes nonce plus sender timestamps to compute RTT and offset.

5) `error` (TV -> phone) 
- Fields: `code` (string), `message` (string). After sending, TV MAY close.

6) `assignSinger` (TV -> phone)

Sent when the user starts a song (Assign Singers overlay) and on reconnect while a song is in progress.

- Fields:
 - `sessionId` (string)
 - `songInstanceId` (string; changes every time a song starts)
 - `role` (`"singer"` or `"spectator"`)
 - If `role=="singer"`:
 - `playerId` (`"P1"` or `"P2"`)
 - `difficulty` (`"Easy" | "Medium" | "Hard"`)
 - `thresholdIndex` (0..7)
 - `effectiveMicDelayMs` (int)
 - `expectedPitchFps` (int; default 50)
 - `startMode` (`"countdown"` or `"live"`)
 - `countdownMs` (int; required if `startMode=="countdown"`)
- Semantics:
 - `role="singer"` instructs the phone to begin streaming frames for the given `playerId` and `songInstanceId`.
 - `role="spectator"` instructs the phone to stop streaming frames (or the TV will ignore them).
 - On song end/quit, TV MUST send `assignSinger` with `role="spectator"` to selected phones (clears assignment).

Validation rules:
- Unknown `type`: ignore + warn (except during handshake; handshake failures are fatal).
- `protocolVersion` mismatch: send `error(code="PROTOCOL_MISMATCH")` and close.

## 8.3 Pitch Stream Messages

Normative MVP rule: phones MUST NOT send any computed scoring, judgement, combo, or rating values.
Phones send only DSP-derived observations (pitch frames and optional confidence/level telemetry).
The TV is the single source of truth for timeline alignment, note matching, and scoring.


Option A: phone sends `toneValid` + `midiNote` at 50 fps.

**Implementation requirements (MVP)**

`pitchFrame` (phone -> TV)
- Fields (required):
 - `type: "pitchFrame"`
 - `protocolVersion` (int)
 - `playerId` (`"P1"` or `"P2"`)
 - `seq` (uint32, increments by 1 per frame)
 - `tCaptureMs` (phone monotonic ms)
 - `toneValid` (bool) MUST match USDX-style thresholding
 - `midiNote` (int or null) MIDI note number (0..127). The TV MUST translate this to USDX semitone scale as `toneUsdx = midiNote - 36`.

MIDI domain (normative):
- `midiNote=69` corresponds to A4 = 440.0 Hz.
- Mapping from `midiNote` to frequency is:
  `f_hz(midiNote) = 440.0 * 2^((midiNote - 69)/12)`

Phone-side computation (normative):
- If the phone pitch tracker produces an estimated fundamental frequency `f0_hz` (Hz):
  - If `f0_hz <= 0` or unvoiced -> `toneValid=false` and `midiNote=null`.
  - Else compute:
    - `midi_raw = 69 + 12 * log2(f0_hz / 440.0)`
    - `midiNote = clamp(round(midi_raw), 0, 127)`
- If the phone pitch tracker produces a semitone index directly, it MUST be converted/clamped to the same [0..127] domain.

Voicing/thresholding (normative):
- `maxAmp` definition (normative): normalized peak amplitude for the audio window that produced the pitch estimate for this frame.
  - If input is 16-bit signed PCM, compute `maxAmp = clamp(max(abs(sample_i)) / 32768.0, 0, 1)` over the window.
  - If input is floating-point samples in [-1..1], compute `maxAmp = clamp(max(abs(sample_i)), 0, 1)`.
- The TV selects a noise threshold via `thresholdIndex` (0..7) and sends it in `assignPlayer`/`assignSinger`.
- The phone MUST compute `toneValid` using the following thresholds on normalized peak amplitude `maxAmp` (0..1):
  - thresholdValueByIndex = [0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.40, 0.60]
  - `toneValid = (maxAmp >= thresholdValueByIndex[thresholdIndex]) AND (pitch_estimate_succeeded)`
- When `toneValid=false`, the phone MUST set `midiNote=null` (or omit it).

Receiver semantics (normative):
- The receiver MUST NOT interpret any specific `midiNote` value (including 0) as silence. Silence/unvoiced is represented only by `toneValid=false`.
- Fields (optional but recommended):
 - `maxAmp` (float 0..1) debugging/telemetry only
 - `thresholdIndex` (int 0..7) debugging only

Rate:
- Default 50 fps (one frame every 20 ms). Phone MAY batch multiple frames in a single WebSocket message as `{"type":"pitchBatch","frames":[...]}`.

Validation:
- Drop frames with decreasing `seq` or `tCaptureMs` regressions > 200 ms.
- If no valid frame exists for a scoring beat window, treat as `toneValid=false` (silence).

## 8.4 Versioning and Compatibility

**Implementation requirements (MVP)**

- Define `protocolVersion = 1` for this MVP.
- TV host MUST reject clients whose `hello.protocolVersion != 1` with `error(code="PROTOCOL_MISMATCH")` and close.
- Backward/forward compatibility is out of scope for MVP; future versions must increment `protocolVersion` and maintain a compatibility table.

# 9. Time Sync, Jitter Handling, and Auto Delay

## 9.1 Defaults

These defaults are chosen to be playable on typical home WiFi while keeping perceived A/V sync acceptable for karaoke (not esports).

- Clock sync ping/pong: every **2s** per phone; compute a phone->TV `clockOffsetMs` using the NTP-lite method in Section 9.1.1 and smooth using the median of the last 5 samples.

- Pitch frame rate: **50 fps** (20ms interval). If phone cant sustain it, allow 25 fps but TV scoring must still sample at detection beats.

- Jitter buffer (TV):
 - Target playout delay: **220ms**
 - Max playout delay cap: **450ms**
 - Frames arriving later than cap are dropped (treated as silence).

- Scoring sample selection:
 - For each detection beat time, use the **most recent pitch frame at or before** that time.
 - If the newest available frame is older than **120ms**, treat as `toneValid=false` for that beat (prevents stale pitch scoring after stalls).

- Silence / missing frames:
 - Missing or invalid frames are treated as `toneValid=false` (no scoring; rap also requires `toneValid=true`).

- Disconnect:
 - No pause; disconnected player scores 0 until reconnect.

### 9.1.1 Clock Sync (NTP-lite, deterministic)

Goal: map each phone pitch frame `tCaptureMs` (phone monotonic milliseconds) into the TV monotonic time domain for jitter buffering and lateness estimation.

Clock model:
- Phone and TV clocks are independent monotonic timers.
- We estimate `clockOffsetMs` such that:
  - `tTvEstMs = tPhoneMs + clockOffsetMs`

Messages (normative; reuse existing ping/pong envelope):
- `ping` (TV -> phone):
  - `tTvSendMs` (TV monotonic ms at send)
- `pong` (phone -> TV):
  - `tTvSendMs` (echo)
  - `tPhoneRecvMs` (phone monotonic ms at receive of ping)
  - `tPhoneSendMs` (phone monotonic ms at send of pong)
  - `tTvRecvMs` (TV monotonic ms at receive of pong; filled by TV at receipt)

Per-sample computation (normative):
- Let:
  - `t1 = tTvSendMs`
  - `t2 = tPhoneRecvMs`
  - `t3 = tPhoneSendMs`
  - `t4 = tTvRecvMs`
- Round-trip delay estimate:
  - `rttMs = (t4 - t1) - (t3 - t2)`
- Offset estimate (TV time minus phone time):
  - `offsetMs = ((t2 - t1) + (t3 - t4)) / 2`

Sample selection/smoothing (normative, simplest reliable):
- Keep the last 5 samples per phone.
- Discard samples with `rttMs < 0` or `rttMs > 2000`.
- Choose the sample with the smallest `rttMs` as the current `clockOffsetMs` (best-of-N reduces WiFi jitter).

Usage (normative):
- When a `pitchFrame` arrives, compute:
  - `frameTimestampPhoneMappedToTv = tCaptureMs + clockOffsetMs`
- `arrivalTimeTv` is the TV monotonic receive time of the `pitchFrame`.
- `latenessMs = arrivalTimeTv - frameTimestampPhoneMappedToTv`.

## 9.2 Auto Mic Delay Adjust (ON by default)

Mic delay is applied only to scoring sample timing by shifting the scoring sample time by `micDelayMs` (see Section 5.1, `CurrentBeatD`).

**Auto-adjust algorithm (MVP-defined; ON by default)**
- Maintain per-phone `effectiveMicDelayMs` in [0, 400].
- Every 2s, compute lateness samples from the last 10s:
 - `latenessMs = (arrivalTimeTv - (frameTimestampPhoneMappedToTv))`
- If the median lateness bias is stable and `abs(median) > 80ms`, nudge:
 - If frames are **late** (median > 0) -> **increase** `effectiveMicDelayMs` by +10ms (pull scoring window earlier).
 - If frames are **early** (median < 0) -> **decrease** `effectiveMicDelayMs` by 10ms.
- Apply cooldown: at most one nudge per 10s window.

**Reset behavior**
- Reset `effectiveMicDelayMs` to baseline (user setting or 0) when:
 - phone reconnects,
 - song changes,
 - clock sync is re-established after a long gap (>5s).

# 10. UI Screens and Flows

This section is normative for MVP UI and navigation on Android TV.

## 10.1 Global navigation and input

- Primary input is TV remote (DPAD + OK/Enter + Back).
- **Back** behavior:
 - From Song List: exits app (or returns to Android launcher).
 - From Settings: returns to Song List.
 - From modal dialogs/overlays (Search, Assign Singers): closes overlay and returns to Song List.
 - From Singing: opens Pause overlay (Resume / Quit to Song List).
- **OK/Enter** selects highlighted item.
- DPAD navigates focus in lists and menus.
- If a software keyboard is shown (Search), Back closes keyboard first, then overlay.

## 10.2 Song preview playback

This section defines the MVP behavior for Song List preview playback (Section 3.4) and the related Preview Volume setting (10.4.3).

**When preview plays (normative)**
- A preview MAY start only when a song row is focused and focus remains unchanged for **600 ms**.
- Preview MUST stop immediately when:
 - focus moves to a different song row
 - Search overlay opens
 - Settings opens
 - Assign Singers opens
 - Singing starts

**What plays (normative)**
- Preview duration: **10 seconds**.
- Preview start time:
 - `#PREVIEWSTART` if present
 - otherwise `#START` if present
 - otherwise 0.0 seconds (implementations MAY choose the first note start time)

**Concurrency and audio routing (normative)**
- Preview MUST NOT overlap with full song playback.
- Preview volume uses **Settings > Audio > Preview Volume**. A value of 0 MUST result in silence (effectively disabling preview).

## 10.3 Assign Singers overlay (per-song)

**Purpose**
- On selecting a song, assign the song to one or two connected phones (singers).

**Fields**
- Singer 1 device: required (list of connected phones).
- Singer 2 device: optional.
- Difficulty per singer: Easy / Medium / Hard.
- If duet:
 - If two singers are selected: assign Singer 1 to P1 and Singer 2 to P2; provide a **Swap Parts** action that swaps which device sings P1 vs P2.
 - If only one singer is selected: allow selecting which duet part is sung (P1 or P2).

**Gating rules**
- Duet songs:
 - Singer 1 required.
 - Singer 2 optional.
- Non-duet songs:
 - Singer 1 required.
 - Singer 2 optional; if selected, both singers sing the same track and are scored independently.

**Empty/error states (normative)**
- If no phones are connected, show a blocking message "No phones connected" and a primary action to open Settings > Connect Phones.

**Actions**
- Start: begins countdown then singing.
- Cancel/Back: returns to Song List.

**Wireframes (TV modal, spec-only interactions)**
```text
Non-duet song

+--------------------------------------------------------------------------------+
| ASSIGN SINGERS                                               Song: <Artist> — <Title> |
+--------------------------------------------------------------------------------+
| Singer (required)                                                              |
|  Phone:      [ Pixel-7 ▾ ]   (dropdown lists connected phone names)            |
|  Difficulty: [ Medium ▾ ]                                                      |
+--------------------------------------------------------------------------------+
| [Start]   [Cancel]                                                             |
+--------------------------------------------------------------------------------+
| Hints: OK=Change/Select   Back=Cancel                                           |
+--------------------------------------------------------------------------------+

Duet song

+--------------------------------------------------------------------------------+
| ASSIGN SINGERS (DUET)                                      Song: <Artist> — <Title> |
+--------------------------------------------------------------------------------+
| Singer 1 (P1)                                Singer 2 (P2)                      |
|  Phone: [ Pixel-7 ▾ ]                        Phone: [ (none) ▾ ] (optional)    |
|  Difficulty: [ Medium ▾ ]                    Difficulty: [ Medium ▾ ]          |
|                                                                                |
| If Singer 2 is (none):  Solo duet part:  (• P1) (  P2)                         |
| If both singers selected:  [Swap Parts]                                        |
+--------------------------------------------------------------------------------+
| [Start]   [Cancel]                                                             |
+--------------------------------------------------------------------------------+
| Hints: OK=Select   Back=Cancel                                                  |
+--------------------------------------------------------------------------------+

Blocking state (no phones connected)

+--------------------------------------------------------------------------------+
| ASSIGN SINGERS                                                                  |
+--------------------------------------------------------------------------------+
| ⚠ No phones connected.                                                         |
|   Connect phones in Settings to sing.                                          |
|                                                                                |
| [Open Settings > Connect Phones]   [Cancel]                                    |
+--------------------------------------------------------------------------------+
```

**Protocol side effects (normative)**
- On Start, TV sends `assignSinger` to each connected phone:
 - Selected devices get `role="singer"` with `playerId`:
  - For non-duet songs: Singer 1 -> `P1`; if Singer 2 selected -> `P2`.
  - For duet songs:
   - If two singers selected: Singer 1 -> `P1`, Singer 2 -> `P2` (swapped if the user selects Swap Parts).
   - If one singer selected: `P1` or `P2` based on the user's duet-part selection.
 - Non-selected devices MAY receive `role="spectator"` (or receive no message).
- When a song ends or user quits:
 - TV sends `assignSinger` with `role="spectator"` (clears assignment).
- Countdown mapping (from Settings > Gameplay):
 - If Ready countdown is ON: send `startMode="countdown"` and `countdownMs = countdownSeconds*1000`.
 - If OFF: send `startMode="live"` and omit `countdownMs`.
## 10.4 Settings Screen

Settings is a simple list of items; selecting one opens a sub-screen.

- Connect Phones
- Song Library
- Audio
- Scoring Timing
- Gameplay
- Video
- Debug (optional)

**Wireframe (TV Settings root)**
```text
+--------------------------------------+
| SETTINGS                              |
|  > Connect Phones                     |
|    Song Library                       |
|    Audio                              |
|    Scoring Timing                     |
|    Gameplay                           |
|    Video                              |
|    Debug (optional)                   |
+--------------------------------------+
| Hints: OK=Open   Back=Return          |
+--------------------------------------+
```

### 10.4.1 Settings > Connect Phones

**Purpose**
- Allow phones to connect via QR/code.
- Show list of connected devices.

**UI**
- QR code + short code.
- Device roster list:
 - display name (editable label), connection status.
 - Optional: latency indicator.

**Actions**
- End session (confirm): invalidates the current session token, disconnects all phones, clears slot assignments, and immediately creates a new session in state Open.
- Rename device: opens a rename dialog (TV on-screen keyboard), updates the stored label for that `clientId`.
- Kick device: confirm then disconnect.
- Forget device: confirm then remove the stored label for that `clientId` and disconnect.

**Wireframe (Connect Phones)**
```text
+--------------------------------------------------------------------------------+
| SETTINGS > CONNECT PHONES                                                      |
+--------------------------------------------------------------------------------+
| Join this session:                                                             |
|   [   QR CODE   ]             Code: ABCD                                       |
|                                                                                |
| Connected devices (up to 10):                                                  |
|  > Pixel-7        Connected                                                    |
|    iPhone-13      Connected                                                    |
|    ...                                                                         |
|                                                                                |
| Actions on selected device:  [Rename] [Kick] [Forget]                           |
| Session: [End session] (confirm)                                               |
+--------------------------------------------------------------------------------+
| Hints: OK=Select/Action   Back=Return                                          |
+--------------------------------------------------------------------------------+
```

**Wireframe (confirm dialog; default focus Cancel)**
```text
+--------------------------------------+
| CONFIRM                              |
| Kick <DeviceName>?                   |
|                                      |
|  > Cancel     OK                     |
+--------------------------------------+

+--------------------------------------+
| CONFIRM                              |
| Forget <DeviceName>?                 |
|                                      |
|  > Cancel     OK                     |
+--------------------------------------+

+--------------------------------------+
| CONFIRM                              |
| End session?                         |
|                                      |
|  > Cancel     OK                     |
+--------------------------------------+
```

### 10.4.2 Settings > Song Library

This is the Add songs workflow.

- Button: **Add songs folder**
 - Opens SAF folder picker.
 - On success: persist permission and add root.
- Root list shows each root with:
 - status (OK / unavailable), last scan, song count.
 - If a root is unavailable, the UI MUST offer a recovery action ("Re-grant access") that re-opens the SAF folder picker for that root and replaces the persisted permission URI on success.
- Actions:
 - Rescan all
 - Rescan root
 - Remove root (confirm)

**Rescan UX (normative)**
- While scanning, the UI MUST show an in-progress status (e.g., "Scanning…") and MUST remain responsive.
- The user MUST be able to cancel an in-progress rescan via Back; cancellation leaves the last successful index intact.

**Wireframe (Song Library while scanning; spec-only interactions)**
```text
+--------------------------------------------------------------------------------+
| SETTINGS > SONG LIBRARY                                                        |
+--------------------------------------------------------------------------------+
| Status: Scanning…   (Back = Cancel)                                            |
|                                                                                |
| Roots                                                                           |
|  > /storage/.../SongsA     OK          last scan: 2026-01-27   songs: 123       |
|    /storage/.../SongsB     UNAVAILABLE last scan: 2026-01-20   songs:  87       |
|        [Re-grant access]                                                       |
|                                                                                |
+--------------------------------------------------------------------------------+
```

**Scan issues (normative)**
- The Song Library screen MUST provide a way to export invalid-song diagnostics captured during scanning (Section 3.2).
- Export MUST include: song path, error reason, and error line number.
- The UI MAY show an in-app list, but MVP only requires an export action.

**Wireframe (Song Library)**
```text
+--------------------------------------------------------------------------------+
| SETTINGS > SONG LIBRARY                                                        |
+--------------------------------------------------------------------------------+
| [Add songs folder]                                                             |
|                                                                                |
| Roots                                                                           |
|  > /storage/.../SongsA     OK          last scan: 2026-01-27   songs: 123       |
|    /storage/.../SongsB     UNAVAILABLE last scan: 2026-01-20   songs:  87       |
|        [Re-grant access]                                                       |
|                                                                                |
| Actions: [Rescan all]  [Rescan root]  [Remove root]                            |
| Diagnostics: [Export invalid-song diagnostics]                                 |
+--------------------------------------------------------------------------------+
| Hints: OK=Select/Action   Back=Return                                          |
+--------------------------------------------------------------------------------+
```

### 10.4.3 Settings > Audio

- **Preview Volume** (normative):
 - Slider 0100.
 - Applies only to Song List preview playback (10.2).
- Optional: Music volume (if you do not rely on system volume).

**Wireframe (Audio)**
```text
+--------------------------------------+
| SETTINGS > AUDIO                      |
+--------------------------------------+
| Preview Volume: [=====|-----]  60     |
| (Optional) Music Volume: [====|----]  |
+--------------------------------------+
| Hints: Left/Right=Adjust  Back=Return |
+--------------------------------------+
```

### 10.4.4 Settings > Scoring Timing

- Manual mic delay baseline (ms).
- Auto mic delay adjust ON/OFF (and status).
- These settings affect the TV scoring timeline (Section 9).

**Wireframe (Scoring Timing)**
```text
+--------------------------------------+
| SETTINGS > SCORING TIMING             |
+--------------------------------------+
| Manual mic delay (ms):   0            |
| Auto mic delay adjust:   ON           |
| Status:                  Calibrated   |
+--------------------------------------+
| Hints: OK=Toggle/Edit  Back=Return    |
+--------------------------------------+
```

### 10.4.5 Settings > Gameplay

- Line bonus ON/OFF (default ON).
- Ready countdown before song start: ON/OFF (default ON).
- Countdown length (seconds): integer 110 (default 3). Countdown ticks at 1 Hz: N, N-1, , 1, then start.
- Optional: show pitch bars ON/OFF (visual only).

**Wireframe (Gameplay)**
```text
+--------------------------------------+
| SETTINGS > GAMEPLAY                   |
+--------------------------------------+
| Line bonus:             ON            |
| Ready countdown:        ON            |
| Countdown seconds:      3             |
| Show pitch bars:        ON            |
+--------------------------------------+
| Hints: OK=Toggle/Edit  Back=Return    |
+--------------------------------------+
```

### 10.4.6 Settings > Video

- Video enabled ON/OFF (if disabled always use background/visualization fallback).

**Wireframe (Video)**
```text
+--------------------------------------+
| SETTINGS > VIDEO                      |
+--------------------------------------+
| Video enabled:          ON            |
+--------------------------------------+
| Hints: OK=Toggle  Back=Return         |
+--------------------------------------+
```

## 10.5 Singing Screen

**Minimum layout**
- Lyrics line with progressive highlight.
- Pitch bars (or equivalent) for each active singer (1 or 2).
- Per-singer score: current total (and optionally note/golden breakdown).
- If a singer disconnects: show Disconnected indicator for that lane and stop increasing that singer's score while disconnected; on reconnect within the same session, scoring resumes (Section 7.4).

**Countdown**
- Countdown before scoring begins is controlled by Settings > Gameplay:
 - If Ready countdown is ON: show N-second countdown at 1 Hz (N from setting) then begin scoring.
 - If OFF: begin scoring immediately.
- If a required singer disconnects during countdown: cancel start and return to Assign Singers with an error message.

**Pause**
- Back opens Pause overlay:
 - Resume
 - Quit to Song List (confirm; clears assignment and stops playback). The confirm dialog MUST default focus to Cancel.

**Wireframes (USDX-aligned, spec-only interactions)**
```text
Active singing screen (composition matches USDX)

+--------------------------------------------------------------------------------+
|                          (FULLSCREEN VIDEO / BACKGROUND)                       |
|                                                                                |
| P1 [badge]                                                                     |
|  ───────────────────────────────────────────────────────────────────────────   |
|   [note bars / pitch lane P1]                                                  |
|                                                                +--------+      |
|                                                                | 00710  |      |
|                                                                +--------+      |
|                                                                perfect!        |
|                                                                                |
| P2 [badge]                                                                     |
|  ───────────────────────────────────────────────────────────────────────────   |
|   [note bars / pitch lane P2]                                                  |
|                                                                +--------+      |
|                                                                | 00720  |      |
|                                                                +--------+      |
|                                                                perfect!        |
|                                                                                |
+--------------------------------------------------------------------------------+
| Lyrics (USDX style: active syllables highlighted)                               |
|   CUz this life is too short                                                   |
|   to live it just for you                                                      |
+--------------------------------------------------------------------------------+
|                                                                      00:35     |
+--------------------------------------------------------------------------------+

Countdown overlay (before playback starts; 1 Hz)

+--------------------------------------------------------------------------------+
|                                                                                |
|                                     3                                          |
|                                                                                |
+--------------------------------------------------------------------------------+
(then 2, 1, 0; at 0 playback + scoring start)

Pause overlay (Back)

+--------------------------------------+
| PAUSED                               |
|  > Resume                            |
|    Quit to Song List                 |
+--------------------------------------+

Quit confirm (default focus Cancel)

+--------------------------------------+
| CONFIRM                              |
| Quit to Song List?                   |
|                                      |
|  > Cancel     OK                     |
+--------------------------------------+
```

## 10.6 Results

### 10.6.1 Results (post-song)

Show per singer:
- Notes score, Golden score, Line bonus, Total (tens-rounded per USDX rules).
- If disconnected mid-song: show a Disconnected indicator and the total disconnected time (and/or number of disconnect intervals) for that singer.

Actions:
- MVP has **no song queue**; returning to Song List is required to start another song.
- Back to Song List
- Play again (re-opens Assign Singers for the same song)

**Wireframe (USDX Song Punkte layout; spec-only actions)**
```text
+--------------------------------------------------------------------------------+
| Song Punkte                                                                    |
| <Artist> — <Title>                                                             |
+--------------------------------------------------------------------------------+
| P1: <PhoneName>                                  | Comparison |     P2: <PhoneName> |
|                                                                                |
| Notes score        00000                          |█████       |   Notes score        00000 |
| Golden score       00000                          |███████     |   Golden score       00000 |
| Line bonus         00000                          |████        |   Line bonus         00000 |
|                                                                                |
| TOTAL             00000                           |██████      |   TOTAL             00000 |
|                                                                                |
+--------------------------------------------------------------------------------+
| [Play Again]   [Back to Song List]                                             |
+--------------------------------------------------------------------------------+
```

# Appendix A: Supported Tags Reference

This appendix is **normative** and is intended to be a single reference table for:
- supported `.txt` header tags and their semantics
- supported body tokens/line types and their grammar
- defaults and version/deprecation behavior

Unless explicitly stated otherwise, the tag semantics below follow **USDX** (`TSong.ReadTXTHeader` / `TSong.LoadSong`).

## A.1 Header tags (USDX-aligned)

Legend:
- Req: required for song validity
- Since/Until: format version applicability. USDX defaults missing `#VERSION` to legacy `0.3.0`.
- Gameplay impact: whether it changes timing/scoring (vs metadata only)

| Tag | Req | Type | Units | Since/Until | Default | Gameplay impact | Normative behavior |
|---|---:|---|---|---|---|---|---|
| `#VERSION` | no | string | - | all | (absent → `0.3.0`) | none | If present, MUST parse as dotted numeric version (e.g., `1.0.0`) or song is invalid. Supported versions are `< 2.0.0`; if `>= 2.0.0` song is invalid (Section 4.3). |
| `#ENCODING` | legacy-only | string | - | `< 1.0.0` only | impl default | none | For `>= 1.0.0`, MUST be ignored and UTF-8 assumed (USDX behavior). |
| `#TITLE` | yes | string | - | all | - | display only | Required; missing/empty invalidates song. |
| `#ARTIST` | yes | string | - | all | - | display only | Required; missing/empty invalidates song. |
| `#AUDIO` | yes (preferred) | string | relative path | `>= 1.0.0` | - | timing (playback clock) | If present, takes precedence over `#MP3`. Referenced file MUST exist or song invalid (Section 3.2/4.3). |
| `#MP3` | yes (fallback) | string | relative path | all | - | timing (playback clock) | Used as audio path for legacy format (`#VERSION` absent or `< 1.0.0`) and as fallback when `#AUDIO` is absent in `#VERSION >= 1.0.0`. Referenced file MUST exist or song invalid. |
| `#BPM` | yes | float | file BPM | all | - | timing/scoring | Required and MUST be non-zero. Internal BPM = `BPM_file * 4` (Section 5.2). |
| `#GAP` | no | float | ms | all | `0` | timing/scoring | Shifts highlight/scoring cursors (Section 5.1). |
| `#START` | no | float | sec | all | `0` | timing (trim) | Audio start trim (Section 5.3). |
| `#END` | no | float | sec | all | `0` | timing (trim) | Audio end trim (Section 5.3). |
| `#VIDEOGAP` | no | float | ms | all | `0` | A/V sync only | Video offset relative to audio (rendering). |
| `#VIDEO` | no | string | relative path | all | unset | none | Optional; missing file is non-fatal in USDX; implementations MAY warn. |
| `#COVER` | no | string | relative path | all | unset | none | Optional cover image. |
| `#BACKGROUND` | no | string | relative path | all | unset | none | Optional background image. |
| `#INSTRUMENTAL` | no | string | relative path | all | unset | none | Optional instrumental track. |
| `#YEAR` | no | int | year | all | `0` | none | Optional metadata year. |
| `#GENRE` | no | string (multi) | - | all | empty | none | Optional multi-valued metadata used for filtering/sorting in USDX. |
| `#EDITION` | no | string (multi) | - | all | empty | none | Optional multi-valued metadata used for filtering/sorting in USDX. |
| `#CREATOR` | no | string (multi) | - | all | empty | none | Optional multi-valued metadata used for filtering/sorting in USDX. |
| `#LANGUAGE` | no | string (multi) | - | all | empty | none | Optional multi-valued metadata used for filtering/sorting in USDX. |
| `#TAGS` | no | string (multi) | - | `>= 1.0.0` | empty | none | Optional multi-valued metadata; USDX parses only for `>= 1.0.0`. |
| `#RESOLUTION` | legacy-only | int | ticks/beat | `<= 1.0.0` only | USDX default | timing (legacy) | Deprecated for `>= 1.0.0` (ignored). Invalid values reset to USDX default. |
| `#NOTESGAP` | legacy-only | int | ticks | `<= 1.0.0` only | `0` | timing (legacy) | Deprecated for `>= 1.0.0` (ignored). |
| `#RELATIVE` | legacy-only | YES/NO | - | `<= 1.0.0` only | `NO` | timing (legacy) | If `YES`, enables legacy relative beat-offset semantics (Section 4.3). For `>= 1.0.0`, song is invalid (USDX behavior). |
| `#PREVIEWSTART` | no | float | sec | all | `0` | none | Optional preview cue point (UI). |
| `#MEDLEYSTARTBEAT` | no | int | beats | all | unset | none | Only allowed when `RELATIVE` is false (USDX behavior). |
| `#MEDLEYENDBEAT` | no | int | beats | all | unset | none | Only allowed when `RELATIVE` is false (USDX behavior). |
| `#CALCMEDLEY` | no | OFF/ON | - | all | ON | none | Controls medley auto-calc in USDX. |
| `#DUETSINGERP1` | legacy-only | string | - | `<= 1.0.0` only | unset | none | Deprecated for `>= 1.0.0` (ignored); legacy duet singer metadata. |
| `#DUETSINGERP2` | legacy-only | string | - | `<= 1.0.0` only | unset | none | Deprecated for `>= 1.0.0` (ignored); legacy duet singer metadata. |
| `#P1` | no | string | - | all | unset | none | Duet singer display name for Player 1. |
| `#P2` | no | string | - | all | unset | none | Duet singer display name for Player 2. |

### A.1.1 Unknown/unsupported header tags
Unknown header tags MUST be preserved as `ParsedSong.header.customTags` (Appendix C) in the order encountered.

## A.2 Body tokens (USDX-aligned)

All body lines are tokenized by the first non-space character. Unknown tokens MUST be ignored with a warning diagnostic unless they cause numeric-parse failure for a recognized token (Section 4.3).

### A.2.1 End of song
| Token | Grammar | Meaning |
|---|---|---|
| `E` | `E` | End of song data. Parsing stops. |

### A.2.2 Duet track selector
| Token | Grammar | Meaning |
|---|---|---|
| `P1` / `P2` | `P <n>` where `n ∈ {1,2}` | Switch active track for subsequent note/sentence lines. If `n` is not 1 or 2, song is invalid (`ERROR_CORRUPT_SONG_INVALID_DUET_MARKER`). |

### A.2.3 Notes
| Token | Grammar | NoteType |
|---|---|---|
| `:` | `: <startBeat> <duration> <tone> <lyric>` | Normal |
| `*` | `* <startBeat> <duration> <tone> <lyric>` | Golden |
| `F` | `F <startBeat> <duration> <tone> <lyric>` | Freestyle |
| `R` | `R <startBeat> <duration> <tone> <lyric>` | Rap |
| `G` | `G <startBeat> <duration> <tone> <lyric>` | RapGolden |

Rules:
- `duration==0` MUST be converted to `F` (Freestyle) with a warning (USDX behavior; Section 4.3).
- In legacy RELATIVE mode, `startBeat` is offset by `Rel[track]` (Section 4.3).
- The lyric field MAY be empty; implementations MUST preserve it as-authored.

### A.2.4 Sentence/line break
| Token | Grammar | Meaning |
|---|---|---|
| `-` | `- <startBeat> [<delta>]` | Sentence ends; new line begins at `startBeat`. In legacy RELATIVE mode, the optional second integer updates relative offset (USDX behavior). |

### A.2.5 BPM change marker
| Token | Grammar | Meaning |
|---|---|---|
| `B` | `B <startBeat> <bpm>` | Adds a BPM segment starting at `startBeat` (file beat). Internal BPM is `bpm * 4` (Section 5.2). In legacy RELATIVE mode, USDX applies `Rel[0]` to the beat value. |

## A.3 Legacy RELATIVE mode (summary)
If `#RELATIVE:YES` in a legacy song (`#VERSION` absent or `< 1.0.0`):
- Each track maintains a `Rel[track]` beat offset applied to note and `-` start beats.
- BPM change `B` uses `Rel[0]` (track 0) offset.

# Appendix B: Protocol Schemas

This appendix is **normative** and defines JSON Schemas for MVP protocol messages described in Section 8.

Schema notes:
- Schemas use JSON Schema Draft 2020-12.
- `additionalProperties` is set to `false` to keep fixtures deterministic for MVP.

## B.1 Common envelope

All messages are JSON objects and MUST include:
- `type` (string)
- `protocolVersion` (int; MUST be `1` in MVP)
- `tsTvMs` (optional; TV may include)

## B.2 Schemas

### B.2.1 `hello`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "hello",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "protocolVersion", "clientId", "deviceName", "appVersion", "capabilities"],
  "properties": {
    "type": {"const": "hello"},
    "protocolVersion": {"type": "integer", "const": 1},
    "tsTvMs": {"type": "number"},
    "clientId": {"type": "string", "minLength": 8},
    "deviceName": {"type": "string", "minLength": 1},
    "appVersion": {"type": "string", "minLength": 1},
    "capabilities": {
      "type": "object",
      "additionalProperties": true,
      "properties": {
        "pitchFps": {"type": "integer", "minimum": 1}
      }
    }
  }
}
```

### B.2.2 `assignPlayer`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "assignPlayer",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "protocolVersion", "playerId", "thresholdIndex"],
  "properties": {
    "type": {"const": "assignPlayer"},
    "protocolVersion": {"type": "integer", "const": 1},
    "tsTvMs": {"type": "number"},
    "playerId": {"type": "string", "enum": ["P1", "P2"]},
    "thresholdIndex": {"type": "integer", "minimum": 0, "maximum": 7},
    "effectiveMicDelayMs": {"type": "integer", "minimum": 0}
  }
}
```

### B.2.3 `sessionState`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "sessionState",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "protocolVersion", "sessionId", "slots", "inSong"],
  "properties": {
    "type": {"const": "sessionState"},
    "protocolVersion": {"type": "integer", "const": 1},
    "tsTvMs": {"type": "number"},
    "sessionId": {"type": "string", "minLength": 1},
    "slots": {
      "type": "object",
      "additionalProperties": false,
      "required": ["P1", "P2"],
      "properties": {
        "P1": {
          "type": "object",
          "additionalProperties": false,
          "required": ["connected", "deviceName"],
          "properties": {
            "connected": {"type": "boolean"},
            "deviceName": {"type": "string"}
          }
        },
        "P2": {
          "type": "object",
          "additionalProperties": false,
          "required": ["connected", "deviceName"],
          "properties": {
            "connected": {"type": "boolean"},
            "deviceName": {"type": "string"}
          }
        }
      }
    },
    "inSong": {"type": "boolean"},
    "songTimeSec": {"type": "number"}
  }
}
```

### B.2.4 `ping` / `pong` (clock sync)
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$defs": {
    "ping": {
      "title": "ping",
      "type": "object",
      "additionalProperties": false,
      "required": ["type", "protocolVersion", "nonce", "tTvSendMs"],
      "properties": {
        "type": {"const": "ping"},
        "protocolVersion": {"type": "integer", "const": 1},
        "tsTvMs": {"type": "number"},
        "nonce": {"type": "string", "minLength": 1},
        "tTvSendMs": {"type": "integer"}
      }
    },
    "pong": {
      "title": "pong",
      "type": "object",
      "additionalProperties": false,
      "required": ["type", "protocolVersion", "nonce", "tTvSendMs", "tPhoneRecvMs", "tPhoneSendMs"],
      "properties": {
        "type": {"const": "pong"},
        "protocolVersion": {"type": "integer", "const": 1},
        "tsTvMs": {"type": "number"},
        "nonce": {"type": "string", "minLength": 1},
        "tTvSendMs": {"type": "integer"},
        "tPhoneRecvMs": {"type": "integer"},
        "tPhoneSendMs": {"type": "integer"},
        "tTvRecvMs": {"type": "integer"}
      }
    }
  }
}
```

### B.2.5 `error`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "error",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "protocolVersion", "code", "message"],
  "properties": {
    "type": {"const": "error"},
    "protocolVersion": {"type": "integer", "const": 1},
    "tsTvMs": {"type": "number"},
    "code": {"type": "string", "minLength": 1},
    "message": {"type": "string", "minLength": 1}
  }
}
```

### B.2.6 `assignSinger`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "assignSinger",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "protocolVersion", "sessionId", "songInstanceId", "role"],
  "properties": {
    "type": {"const": "assignSinger"},
    "protocolVersion": {"type": "integer", "const": 1},
    "tsTvMs": {"type": "number"},
    "sessionId": {"type": "string", "minLength": 1},
    "songInstanceId": {"type": "string", "minLength": 1},
    "role": {"type": "string", "enum": ["singer", "spectator"]},
    "playerId": {"type": "string", "enum": ["P1", "P2"]},
    "difficulty": {"type": "string", "enum": ["Easy", "Medium", "Hard"]},
    "thresholdIndex": {"type": "integer", "minimum": 0, "maximum": 7},
    "effectiveMicDelayMs": {"type": "integer", "minimum": 0},
    "expectedPitchFps": {"type": "integer", "minimum": 1},
    "startMode": {"type": "string", "enum": ["countdown", "live"]},
    "countdownMs": {"type": "integer", "minimum": 0}
  },
  "allOf": [
    {
      "if": {"properties": {"role": {"const": "singer"}}},
      "then": {"required": ["playerId", "difficulty", "thresholdIndex", "effectiveMicDelayMs", "expectedPitchFps", "startMode"]}
    },
    {
      "if": {"properties": {"startMode": {"const": "countdown"}}},
      "then": {"required": ["countdownMs"]}
    }
  ]
}
```

### B.2.7 `pitchFrame`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "pitchFrame",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "protocolVersion", "playerId", "seq", "tCaptureMs", "toneValid"],
  "properties": {
    "type": {"const": "pitchFrame"},
    "protocolVersion": {"type": "integer", "const": 1},
    "playerId": {"type": "string", "enum": ["P1", "P2"]},
    "seq": {"type": "integer", "minimum": 0},
    "tCaptureMs": {"type": "integer"},
    "toneValid": {"type": "boolean"},
    "midiNote": {"type": ["integer", "null"], "minimum": 0, "maximum": 127},
    "maxAmp": {"type": "number", "minimum": 0, "maximum": 1},
    "thresholdIndex": {"type": "integer", "minimum": 0, "maximum": 7}
  },
  "allOf": [
    {
      "if": {"properties": {"toneValid": {"const": false}}},
      "then": {"properties": {"midiNote": {"type": "null"}}}
    }
  ]
}
```

### B.2.8 `pitchBatch`
```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "pitchBatch",
  "type": "object",
  "additionalProperties": false,
  "required": ["type", "protocolVersion", "frames"],
  "properties": {
    "type": {"const": "pitchBatch"},
    "protocolVersion": {"type": "integer", "const": 1},
    "frames": {
      "type": "array",
      "minItems": 1,
      "items": {"$ref": "#B.2.7"}
    }
  }
}
```
# Appendix C: Parsed Song Model (Normative)

This appendix defines the **normative in-memory representation** of a parsed USDX `.txt` song.

- Implementations MAY choose different class/type names.
- Implementations MUST preserve the fields, semantics, and invariants described here.
- All beats in this model are expressed in **file beats** (the beats used in `.txt` note lines). Conversion to internal beats is defined in Section 5.2.

## C.1 Core entities

### ParsedSong

Required fields:

- `songId` (string): stable identifier of the song instance in the library (e.g., derived from song folder URI + relative path).
- `header` (SongHeader)
- `timing` (SongTiming)
- `tracks` (Track[1..2])
- `diagnostics` (DiagnosticEntry[]) : parse-time diagnostics; MUST include line numbers when available.

Invariants:

- `tracks.length` MUST be `2` if and only if duet mode is detected (Section 4.1: first non-empty body token begins with `P`). Otherwise `tracks.length` MUST be `1`.
- All note events in `tracks[*].lines[*].notes[*]` MUST satisfy `durationBeats >= 1` (duration 0 MUST already be converted to Freestyle per Section 4.3).

### SongHeader

Required fields (mirrors Section 4.2 semantics):

- `title` (string)
- `artist` (string)
- `bpmFile` (float) : file BPM from `#BPM` (before internal conversion)
- `gapMs` (int) : from `#GAP`
- `audio` (string) : resolved audio filename (from `#AUDIO` when `#VERSION >= 1.0.0` and present; otherwise from `#MP3`, USDX behavior)

Optional fields:

- `video` (string|null)
- `cover` (string|null)
- `background` (string|null)
- `p1Name` (string|null) : from `#P1` (duet)
- `p2Name` (string|null) : from `#P2` (duet)
- `relativeMode` (bool) : legacy `<1.0.0` RELATIVE mode (Section 4.3)
- `version` (string) : from `#VERSION` if present; otherwise treated as `0.3.0` for legacy behavior (Section 4.3)
- `customTags` (map<string,string>) : unknown/malformed tags preserved per Section 4.3

### SongTiming

Required fields:

- `bpmChanges` (BpmChange[]) : ordered by increasing `startBeatFile`

Optional/derived fields:

- `startBeatFile` (int|null) : from `#START` if present
- `endBeatFile` (int|null) : from `#END` if present
- `notesGapBeatsFile` (int|null) : from `#NOTESGAP` if present

`BpmChange`:

- `startBeatFile` (int) : beat at which this BPM becomes active (file beats)
- `bpmFile` (float) : BPM value that applies from `startBeatFile` until the next change

Invariants:

- If there are no in-song `B` tokens, `bpmChanges` MUST contain exactly one entry with `startBeatFile=0` and `bpmFile=header.bpmFile`.

### Track

Required fields:

- `trackIndex` (int) : `0` for P1 (or solo), `1` for P2
- `lines` (Line[]) : ordered in encounter order

### Line

A "line" corresponds to a sentence/phrase separated by `-` tokens in the body.

Required fields:

- `lineIndex` (int) : 0-based within track
- `notes` (NoteEvent[]) : ordered by `startBeatFile` then encounter order

Optional/derived fields:

- `startBeatFile` (int) : the `startBeatFile` of the first note in the line (if any)
- `endBeatFileExclusive` (int) : `max(note.startBeatFile + note.durationBeats)` over notes in the line

Invariants:

- Lines MAY be empty (e.g., consecutive `-`), but empty lines MUST NOT affect scoring.

### NoteEvent

Required fields:

- `noteType` (enum) : one of:
  - `Normal`  (token `:`)
  - `Golden`  (token `*`)
  - `Rap`     (token `R`)
  - `RapGolden` (token `G`)
  - `Freestyle` (token `F`)
- `startBeatFile` (int)
- `durationBeats` (int)
- `toneUsdx` (int) : semitone index in USDX scale (`C2 = 0`) (Section 6).
- `lyric` (string) : as authored (may be empty)

Optional/derived fields:

- `endBeatFileExclusive` (int) : `startBeatFile + durationBeats`


# Appendix D: ParsedSongModel Fixture Serialization (Normative)

This appendix defines a **portable JSON** serialization for fixtures that validate:

- TXT parsing output (Appendix C)
- Timing/beat conversion and scoring (Sections 5–6)

The goal is that the same fixtures can be consumed by:

- end-to-end acceptance tests
- unit tests (parser, timing, scoring, clock sync)

## D.1 Fixture folder layout

Each fixture is a directory containing at minimum:

- `song.txt` : the USDX `.txt` under test
- `expected.parsedSong.json` : the expected `ParsedSong` (Appendix C) serialized as JSON

Optional files depending on fixture type:

- `pitchFrames.jsonl` : one JSON object per line (same fields as protocol `pitchFrame`, Section 8.3)
- `expected.score.json` : expected scoring totals and/or per-beat outcomes

## D.2 JSON schema (structural)

The following describes the **required shape** (not a specific JSON-Schema draft).

Top level: `ParsedSong`

- `songId`: string
- `header`: object
  - `title`: string
  - `artist`: string
  - `bpmFile`: number
  - `gapMs`: integer
  - `audio`: string
  - `video`: string|null (optional)
  - `cover`: string|null (optional)
  - `background`: string|null (optional)
  - `p1Name`: string|null (optional)
  - `p2Name`: string|null (optional)
  - `relativeMode`: boolean (optional)
  - `version`: string (optional)
  - `customTags`: object<string,string> (optional)
- `timing`: object
  - `bpmChanges`: array of objects `{ startBeatFile:int, bpmFile:number }`
  - `startBeatFile`: int|null (optional)
  - `endBeatFile`: int|null (optional)
  - `notesGapBeatsFile`: int|null (optional)
- `tracks`: array (length 1 or 2)
  - track object:
    - `trackIndex`: int
    - `lines`: array
      - line object:
        - `lineIndex`: int
        - `notes`: array
          - note object:
            - `noteType`: string (one of `Normal|Golden|Rap|RapGolden|Freestyle`)
            - `startBeatFile`: int
            - `durationBeats`: int
            - `toneUsdx`: int
            - `lyric`: string
- `diagnostics`: array (may be empty)
  - diagnostic entry object:
    - `severity`: string
    - `code`: string
    - `message`: string
    - `lineNumber`: int|null

## D.3 Pitch frame fixture format (`pitchFrames.jsonl`)

Each line is a JSON object with the same required fields as Section 8.3 `pitchFrame`:

- `seq` (uint32)
- `tCaptureMs` (int)
- `toneValid` (bool)
- `midiNote` (int|null)

Implementations MUST treat missing frames for a scoring beat as `toneValid=false` for that beat (Section 9.1).
# Appendix E: Worked Examples (Normative for fixtures)

This appendix provides worked numeric examples to remove ambiguity in:
- timing/beat conversion (Section 5)
- beat stepping and note-window boundaries (Sections 5.2, 6.1)
- scoring normalization, line bonus, and rounding (Sections 6.5–6.6)

These examples are intended to be copied into fixtures by providing:
- `song.txt` (minimal chart)
- optional `pitchFrames.jsonl` (MIDI-based detection stream)
- `expected.score.json` (authoritative intermediate values + expected totals)

## E.1 Static BPM beat cursors (highlight vs scoring)

Given:
- `BPM_file = 120.0`
- `BPM_internal = BPM_file * 4 = 480.0`
- `beatsPerSec = BPM_internal / 60.0 = 8.0`
- `GAPms = 2000`
- `micDelayMs = 100`
- `lyricsTimeSec = 5.0`

Compute (Section 5.1):
- `highlightTimeSec = lyricsTimeSec - (GAPms/1000) = 5.0 - 2.0 = 3.0`
- `scoringTimeSec  = lyricsTimeSec - ((GAPms+micDelayMs)/1000) = 5.0 - 2.1 = 2.9`

Convert time to beats (Section 5.2, static BPM):
- `MidBeat_internal(highlight) = 3.0 * 8.0 = 24.0`
- `CurrentBeat = floor(24.0) = 24`

- `MidBeat_internal(scoring) = 2.9 * 8.0 = 23.2`
- `CurrentBeatD = floor(23.2 - 0.5) = floor(22.7) = 22`

Implication:
- A scoring update from `oldBeatD=19` to `currentBeatD=22` MUST evaluate beats `b = 20, 21, 22` (Section 6.1).

## E.2 Variable BPM example (segment walk + clamp)

Song timing:
- Header: `BPM_file = 120.0` (so `bpm_internal = 480.0`)
- One BPM change token: `B 16 60.0` (file beats)
  - At file beat 16, internal BPM becomes `60.0 * 4 = 240.0`

Segment 0:
- beats: 0..16 (16 file beats)
- `secPerBeat = 60/480 = 0.125`
- `segTime = 16 * 0.125 = 2.0s`

Segment 1:
- beats: 16..∞
- `secPerBeat = 60/240 = 0.25`

Example A: `tSec = 3.0s`
- Consume segment 0: `tSec := 3.0 - 2.0 = 1.0`, accumulated beats = 16
- Inside segment 1: add `tSec * (bpm_internal/60) = 1.0 * (240/60) = 4.0`
- Result: `MidBeat_internal = 16 + 4 = 20.0`

Example B: clamp behavior (Section 5.2, variable BPM)
- If `tSec <= 0`, `TimeSecToMidBeatInternal(tSec) = 0`.

## E.3 Beat stepping and note-window boundary convention

Given:
- `oldBeatD = 10`
- `currentBeatD = 13`

Then (Section 6.1):
- Evaluate beats `b = 11, 12, 13` only.

If a note has:
- `startBeatFile = 11`
- `durationBeats = 2`
- `endBeatExclusive = startBeatFile + durationBeats = 13`

Then (Section 5.2 boundary convention):
- active at `b=11` and `b=12`
- NOT active at `b=13` (end exclusive)

## E.4 Scoring normalization and line bonus (fully-worked minimal song)

Assume:
- Line bonus: ON
- `MaxSongPoints = 9000`
- `MaxLineBonusPool = 1000`

Create a minimal SOLO track (trackIndex=0) with two non-empty lines:

Line 1:
- `: 0 4 0 la`
- `- 4`

Line 2:
- `* 4 4 0 la`
- `- 8`
- `E`

Where (Section 6.2.1):
- Normal (`:`) has `ScoreFactor=1`
- Golden (`*`) has `ScoreFactor=2`

Compute `TrackScoreValue` (Section 6.5):
- Line1 ScoreValue = `4 * 1 = 4`
- Line2 ScoreValue = `4 * 2 = 8`
- `TrackScoreValue = 4 + 8 = 12`

Per-beat points (Section 6.6):
- For Normal hit-beat: `CurBeatPoints = (MaxSongPoints / TrackScoreValue) * 1 = (9000/12) = 750`
- For Golden hit-beat: `CurBeatPoints = (MaxSongPoints / TrackScoreValue) * 2 = (9000/12) * 2 = 1500`

Perfect performance (all eligible beats hit; `toneValid=true`; pitch in range for Normal/Golden):
- Line 1 beats b=0..3: `Player.Score = 4 * 750 = 3000`
- Line 2 beats b=4..7: `Player.ScoreGolden = 4 * 1500 = 6000`
- Note totals = 9000

Line bonus (Section 6.5):
- `NonEmptyLines = 2`
- `LineBonusPerLine = MaxLineBonusPool / NonEmptyLines = 1000 / 2 = 500`

Line 1:
- `MaxLineScore = MaxSongPoints * (Line1ScoreValue / TrackScoreValue) = 9000 * (4/12) = 3000`
- At sentence end: `LineScore = (Score + ScoreGolden) - ScoreLast = 3000 - 0 = 3000`
- `LinePerfection = clamp(LineScore / (MaxLineScore - 2), 0, 1) = clamp(3000/2998, 0, 1) = 1`
- `ScoreLine += LineBonusPerLine * LinePerfection = 500`

Line 2:
- `MaxLineScore = 9000 * (8/12) = 6000`
- `LineScore = 9000 - 3000 = 6000`
- `LinePerfection = clamp(6000/5998, 0, 1) = 1`
- `ScoreLine += 500`

So: `Player.ScoreLine = 1000`

Rounding (Section 6.6):
- `Player.ScoreLineInt = floor(round(ScoreLine)/10)*10 = 1000`
- `ScoreInt = round(Player.Score/10)*10 = 3000`
- Since `ScoreInt < Player.Score` is FALSE, `ScoreGoldenInt = floor(Player.ScoreGolden/10)*10 = 6000`
- `ScoreTotalInt = ScoreInt + ScoreGoldenInt + ScoreLineInt = 10000`

## E.5 Golden rounding direction rule (fractional demonstration)

This example exists only to demonstrate the “golden rounds opposite” rule (Section 6.6).

Assume after accumulation:
- `Player.Score = 4090.909...`
- `Player.ScoreGolden = 100.909...`

Compute:
- `ScoreInt = round(4090.909/10)*10 = 4090`
- Since `ScoreInt < Player.Score` is TRUE, apply opposite rounding:
  - `ScoreGoldenInt = ceil(100.909/10)*10 = 110`

## E.6 Minimal fixture files for E.4 (reference layout)

Fixture directory example: `E4_score_linebonus_perfect/`

- `song.txt` contains the exact 2-line chart from E.4.
- `expected.score.json` contains at least:
  - `MaxSongPoints`, `MaxLineBonusPool`
  - `TrackScoreValue`
  - per-note `CurBeatPoints` values
  - `Score`, `ScoreGolden`, `ScoreLine`, and the tens-rounded ints
  - `ScoreTotalInt`

`pitchFrames.jsonl` is OPTIONAL for E.4 if the harness can inject per-beat hit/miss booleans. If the fixture uses the full scoring pipeline, provide `pitchFrames.jsonl` with `toneValid=true` and `midiNote` matching the target tone for each scoring beat.

# Appendix F: Fixtures Guide and Acceptance Inventory

This appendix documents the fixture **conventions** for this repository: intended on-disk layout, the fixture manifest, and the acceptance fixture inventory (F01–F15) including required subcases and expected files.

## F.1 Scope and goals

Fixtures are intended to be:

- **Deterministic**: expected outputs are asserted via JSON files; dynamic values (timestamps, generated IDs, absolute URIs) are not asserted.
- **Portable**: all paths are relative within the repository.
- **Small**: keep audio files as empty stubs unless the test requires real media; prefer minimal charts.
- **Reusable**: the same fixtures should support both acceptance tests and unit tests.

## F.2 Repository layout (convention for this repo)

All fixture content is located under the repo directory `fixtures/`. The **manifest** (`fixtures/manifest.json`) is the authoritative index for which fixtures exist and where their files live.

Directory conventions (a test harness MUST follow the manifest; directories MAY be absent until fixtures are authored):

- `fixtures/manifest.json` — the fixture manifest (Section F.3).
- `fixtures/song_txt_variants/` — single-song TXT parser variants (S01–S17). Each variant is a small folder typically containing `song.txt` and a media stub file referenced by the song header.
- `fixtures/Fxx_*/` — acceptance fixtures (F01–F15). Each `Fxx_*` fixture is a directory; some fixtures include an internal `songs_root/` tree for recursive discovery.
- `fixtures/E*_*/` — optional worked-example fixtures derived from Appendix E (e.g., `E4_score_linebonus_perfect/`).

Note: The UltraStar Deluxe (USDX) rules/spec do not mandate a repo layout. This layout is a practical convention used by this repository to keep fixtures discoverable and controlled.

## F.3 Fixture manifest (`fixtures/manifest.json`)

The manifest is a **machine-readable index** used by test harnesses to locate fixtures and map them back to spec coverage.

At a high level, it contains:

- `manifestVersion`: manifest schema version.
- `specVersion`: the spec version the manifest was authored against.
- `root`: repository-relative root for fixture paths (typically `fixtures`).
- `fixtures[]`: entries with:
  - `id`: stable fixture ID (e.g., `S01`, `F08`).
  - `name`: short slug.
  - `tags`: categories like `parser`, `timing`, `scoring`, `protocol`.
  - `status`: `implemented` or `planned`.
  - `paths`: pointers to relevant files (either specific file paths such as `songTxt`, or a `fixtureDir` for multi-file fixtures).
  - `covers`: list of spec section references (e.g., `4.2`, `Appendix D`).

Test harness guidance:

- Tests should discover fixtures via the manifest rather than hard-coding directory paths.
- Tests may filter by `tags` and/or `status`.
- Tests should treat `covers` as an informational mapping (coverage reporting), not as runtime logic.

## F.4 Fixture directory types (what files to expect)

### F.4.1 Parse-only fixture (TXT parsing)

A parse-only fixture is a directory containing at minimum:

- `song.txt`
- `expected.parsedSong.json`

This validates TXT parsing into the Parsed Song Model (Appendix C).

### F.4.2 Scoring fixture (timing + scoring)

A scoring fixture is a parse-only fixture plus deterministic scoring inputs/outputs:

- `pitchFrames.jsonl` (recommended for acceptance fixtures)
  - one JSON object per line
  - timestamped via `tCaptureMs`
  - includes at least the protocol-required fields: `seq`, `tCaptureMs`, `toneValid`, `midiNote`
- `expected.score.json`

This validates beat/time conversion (Section 5) and scoring (Section 6). For acceptance, prefer `pitchFrames.jsonl` over synthetic hit/miss injection because it also covers jitter/clock mapping behavior.

### F.4.3 Discovery/index fixture (library scan)

A discovery/index fixture validates **recursive discovery** plus **accept/reject validation** across **multiple songs** (Section 3.2, 4.3).

A typical fixture directory contains:

- `songs_root/` — a directory tree with multiple song folders and their `.txt` files. Each song folder SHOULD be laid out like a real song folder (i.e., `song.txt` plus any referenced media files).
- `expected.discovery.json` — an expected discovery result file **defined by this repository** (not a USDX file format).

`expected.discovery.json` MUST be deterministic and must only assert stable fields.

Minimum schema (repo convention):

```json
{
  "rootRel": "songs_root",
  "songs": [
    {
      "songDirRel": "a/valid_minimal",
      "songTxtRel": "a/valid_minimal/song.txt",
      "isValid": true,
      "invalidReasonCode": null,
      "invalidLineNumber": null
    }
  ]
}
```

Field semantics:

- `songDirRel`: song folder path relative to `songs_root/`.
- `songTxtRel`: path to the `.txt` file relative to `songs_root/`.
- `isValid`: `true` iff the song is accepted by the validation rules.
- `invalidReasonCode`: stable string identifier for the rejection reason when `isValid=false` (see Section 4.3 diagnostics rules). `null` when valid.
- `invalidLineNumber`: 1-based line number when the rejection is attributable to a specific line in `song.txt`; otherwise `null`.

Tests MAY additionally assert other deterministic index fields (e.g., `artist`, `title`, `version`, `hasVideo`) as needed, but should not assert dynamic values.


### F.4.4 Protocol/session fixture (control messages)

Some acceptance fixtures require **control-message transcripts** (e.g., pairing, assignment, reconnect reclaim; Section 8).

A typical fixture directory contains:

- `transcript.jsonl` — a message transcript file **defined by this repository** (not a USDX file format)
- `expected.session.json` — expected outcomes **defined by this repository**

#### `transcript.jsonl` (repo convention)

- UTF-8 text file
- one JSON object per line
- each line represents a single protocol message observed or injected

Minimum schema (repo convention):

```json
{
  "direction": "phone_to_tv",
  "message": { "type": "hello", "clientId": "..." }
}
```

Field semantics:

- `direction`: `phone_to_tv` or `tv_to_phone`.
- `message`: a JSON object that MUST conform to the relevant protocol message schema from Section 8.

Tests MAY include optional fields for ordering/debugging (e.g., `tLocalMs`, `seq`), but they must not be required for correctness unless the fixture explicitly tests ordering.

#### `expected.session.json` (repo convention)

This file asserts only deterministic outcomes. Minimum schema:

```json
{
  "accepted": true,
  "finalAssignments": [
    { "clientId": "...", "singerSlot": 0 }
  ],
  "reclaimed": [
    { "clientId": "...", "reclaimedSingerSlot": 0 }
  ],
  "rejected": [
    { "clientId": "...", "reason": "..." }
  ]
}
```

The `reason` strings must be stable identifiers as defined by the implementation/spec rules for the corresponding scenario.


## F.5 Acceptance fixture inventory (F01–F15)

Each acceptance fixture ID defines a scenario to be covered. Implementations should keep these fixtures small and deterministic.

### F01 — Song discovery + validation acceptance

Verifies:

- recursive `.txt` discovery under a configured root (Section 3.2)
- accept/reject validation rules (Section 3.2, 4.3)
- invalidation diagnostics include a stable `invalidReasonCode` and, when applicable, `invalidLineNumber` (Section 4.3)

Inputs (repo convention):

- `songs_root/` directory tree containing a mix of valid and invalid songs
- `expected.discovery.json` describing per-song deterministic validity/diagnostics

Required subcases (minimum):

- valid song with existing audio file
- invalid: missing required header tag (e.g., `#ARTIST`)
- invalid: missing required audio file referenced by `#AUDIO`/`#MP3`
- **Audio resolution subcases**:
  - `>= 1.0.0`: both `#AUDIO` and `#MP3` present → `#AUDIO` takes precedence
  - legacy `< 1.0.0`: `#MP3` required; `#AUDIO` ignored for resolution
- **Optional asset subcase**: missing optional `#VIDEO`/`#COVER`/`#BACKGROUND` is non-fatal (warn/ignore)

### F02 — Header parsing edge cases

Verifies:

- duplicate tags: last-wins (Section 4.2)
- unknown tags preserved (custom tags) and/or ignored per rules (Section 4.2)
- malformed required tags invalidate (Section 4.3)

Required subcases:

- duplicates (e.g., multiple `#BPM`): last-wins
- unknown tags with and without `:`
- required-tag failures (missing required field; malformed numeric)
- **Encoding subcase**: `>= 1.0.0` UTF-8 forced; legacy honors `#ENCODING` (Section 4.2)
- **Preview start subcase**: `previewStartSec` computed from `#PREVIEWSTART` else `#START` else 0 (Section 3.4, 10.2)

### F03 — Body grammar: token recognition + invalidation rules

Verifies:

- unknown tokens are ignored with a warning; recognized tokens with numeric parse failures invalidate (Section 4.2–4.3)
- duration=0 note is parsed as freestyle (Appendix A)

Required subcases:

- unknown token line
- malformed numeric fields on a recognized note token
- duration=0 note → freestyle note
- **Freestyle scoring subcase**: freestyle yields no pitch-based scoring (Section 6.2)

### F04 — Duet parsing: P1/P2 track routing

Verifies:

- duet detection and track routing via `P1`/`P2` sections (Section 4.2)
- invalid `P` marker rejection (Section 4.3)

Required subcases:

- valid duet with interleaved P1/P2 sections and lyrics
- invalid marker (e.g., `P3`) rejected

### F05 — Legacy RELATIVE mode semantics (<1.0.0)

Verifies:

- RELATIVE mode offsets applied correctly per track (Section 4.2)
- BPM change behavior uses Rel[0] in relative mode (Section 5.2)

Inputs:

- legacy `song.txt` with `RELATIVE:YES`, `-` lines, and `B` BPM lines

### F06 — Beat/time conversion: static BPM

Verifies:

- `lyricsTimeSec ↔ beat` conversion formulas and `CurrentBeat/CurrentBeatD` behavior (Section 5.1–5.2)

Inputs:

- minimal `song.txt` with BPM/GAP/NOTESGAP/micDelay as needed
- `expected.score.json` or a purpose-specific expected-values file containing expected beat values for sample times

### F07 — Beat/time conversion: variable BPM with clamp

Verifies:

- segment-walk conversion across `B` BPM changes (Section 5.2)
- clamp behavior for `tSec <= 0` in variable BPM case (Section 5.2)

### F08 — Scoring beat stepping correctness (interval semantics)

Verifies:

- evaluate beats in `(oldBeatD, currentBeatD]` semantics; note active window `start <= b < end` (Section 6.1–6.2)

Inputs:

- scoring fixture with `pitchFrames.jsonl` and `expected.score.json` containing per-step expectations

### F09 — Pitch tolerance + octave normalization (Normal/Golden only)

Verifies:

- tolerance ranges per difficulty (Section 6.3)
- octave normalization loops by ±12 (Section 6.4)

Inputs:

- scoring fixture whose frames force normalization cases (e.g., detected ±12, ±24)

### F10 — Rap scoring: presence-only gated by toneValid

Verifies:

- rap ignores pitch difference but still requires `toneValid=true` (Section 6.2)

Inputs:

- scoring fixture with rap notes and frames toggling `toneValid` while `midiNote` varies

### F11 — Line bonus + rounding rules

Verifies:

- normalization to 10,000; line bonus pool distribution; tens rounding and golden opposite rounding rule (Section 6.5–6.6; Appendix E)

Inputs:

- minimal chart with ≥2 lines
- `expected.score.json` with intermediate values and final totals

### F12 — Pitch stream message validation + semantics

Verifies:

- required fields for pitch frames (Section 8.3; Appendix D)
- dropping invalid frames (seq decreases, `tCaptureMs` regression)
- silence represented by `toneValid=false` (and `midiNote=null`)

Inputs:

- `pitchFrames.jsonl` containing valid frames plus deliberate invalid regressions

### F13 — Jitter buffer selection + staleness rule

Verifies:

- choose most recent frame `<= scoringTime`
- staleness cutoff forces `toneValid=false` (Section 9)

Inputs:

- timestamped `pitchFrames.jsonl` around scoring instants and one stale case

### F14 — Clock sync NTP-lite (offset computation + best-of-N)

Verifies:

- RTT/offset math
- discard invalid RTT
- choose smallest RTT from last N and apply offset mapping (Section 9)

Inputs (repo convention):

- `transcript.jsonl` (or `clock_sync_samples.json`) containing ping/pong tuples
- expected computed `clockOffsetMs` values in an expected-output JSON

### F15 — Session lifecycle: hello/assignSinger + reconnect reclaim

Verifies:

- reconnect reclaim via stable `clientId` (Section 7.4)
- role resumption; rejection behavior when session is full and cannot match (Section 7.1–7.4)

Inputs (repo convention):

- `transcript.jsonl` of control messages (hello, assignSinger, disconnect, reconnect)
- expected outcome JSON describing assigned slots and accept/reject decisions

### Notes on subcases

Subcases listed above are requirements within the corresponding fixture ID. They should be implemented as small additional variants inside the same fixture directory (e.g., additional songs in `songs_root/` for F01, or additional minimal charts/frames for scoring fixtures) rather than adding new top-level fixture IDs.
