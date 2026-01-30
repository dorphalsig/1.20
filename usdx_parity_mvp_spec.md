Android Karaoke Game
USDX Parity MVP Functional Specification

Version: 0.3

Date: 2026-01-25

Owner: TBD

Status: Draft



# Change Record (rolling, last 4 hours, Europe/Berlin)

| Timestamp | Author | Changes |
| --- | --- | --- |
| 2026-01-30 04:49 CET | TBD | Align pitch protocol with USDX: phone sends MIDI note numbers; TV derives USDX tone scale (C2=0) and applies USDX octave normalization. |
| 2026-01-30 04:47 CET | TBD | Align variable-BPM time->beat conversion with USDX: clamp tSec<=0 to beat 0. |
| 2026-01-30 04:46 CET | TBD | Align beat model with USDX: do not scale note/sentence/B-line beats; only scale BPM by 4. |
| 2026-01-30 04:45 CET | TBD | Add rolling change record; future edits must append here and prune entries older than 4 hours. |



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
- 11. Parity Test Suite
  - 11.1 Golden Parsing Fixtures
  - 11.2 Golden Scoring Fixtures
  - 11.3 Live Network Tests
  - 11.4 Test Report Format
- Appendix A: Supported Tags Reference
- Appendix B: Protocol Schemas
- Appendix C: Fixture Inventory
  - C.1 gangnamstyle-normal-5s (protocol + ordered frames)
  - C.2 gangnamstyle-rap-5s (unvoiced frames + large batches)
  - C.3 Future scoring fixtures (reserved)


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

Paritiy MVP PASS requires all golden parsing and scoring fixtures to match expected results, plus functional pairing and play flows operating reliably on typical home Wi-Fi (see Section 11).

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
- `#BPM` MUST be present and parseable as a positive floating-point number.
- A required audio reference tag MUST be present:
  - If `#VERSION` is absent or < 1.0.0, `#MP3` MUST be present and non-empty.
  - If `#VERSION` is >= 1.0.0, `#AUDIO` MUST be present and non-empty.
  (Audio tag precedence and resolution is defined in Section 4.2.)

2) Required audio file exists
- The audio filename resolved by the rules in Section 4.2 MUST exist in the same directory as the `.txt` (unless the resolved value is an absolute URI supported by the platform; if absolute URIs are not supported in MVP, treat them as missing).

3) Notes section parses without fatal errors
- The notes/body section MUST be parsed according to Section 4.1 and Section 4.3.
- Unknown tokens and recoverable grammar issues MUST be handled per Section 4.3 (warn and continue).
- Any fatal numeric parse error for a recognized token MUST reject the song entry.

4) Each track has at least one non-empty sentence after cleanup
After body parsing completes, validation MUST ensure each parsed track (single track, or both tracks for duet) contains singable structure:

- The track MUST contain at least one sentence delimiter (`-`), resulting in at least one sentence/line object.
  - If a track contains zero sentence delimiters, reject with reason `ERROR_CORRUPT_SONG_NO_BREAKS`.

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

 Define which fields must be stored to render Song Select without full re-parse (e.g., title/artist, flags, URIs, modified times, validation status, duet labels).


## 3.4 Song List (Landing Screen)

**Purpose**
- Always the landing screen (even if library is empty).
- Displays songs sorted by **Artist -> Album -> Title**.
- MVP has **no song queue/playlist**; only one song is selected and played at a time.

**Header actions**
- **Settings** button: opens Settings screen.
- **Search** button: opens Search overlay (see Section 3.5).

**Pairing (on landing)**
- The landing screen MUST display only the session join QR code and short join code (no roster on the landing screen).

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
|   choose your song                                                             |
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
|  Pair / Join (landing: only QR + code)                                         |
|     [  QR  ]     Code: ABCD                                                    |
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
 - Format 1.0.0: `#AUDIO:` preferred; if present it overrides `#MP3:`.
 - Older formats: `#MP3:` is used.
 - Audio file must exist, otherwise load fails.

### Timing/alignment tags
- `#GAP:` millisecond offset used as the lyrics/audio time origin for beat/time conversions (see Section 5.1).
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

- Header lines are read from the top of the file while the line is either empty or starts with `#`.
- Tag names are case-insensitive; matching MUST be performed on `Uppercase(Trim(TagName))`.
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
- Unsupported `#VERSION` -> invalid.
- For `VERSION >= 1.0.0`, treat file as UTF-8; ignore `#ENCODING` with a warn/info log.
- For legacy versions, apply `#ENCODING` if present; decode failure -> invalid.

**Logging**
- All invalidation MUST include a concise reason string suitable for display in debug invalid songs listing.

# 5. Timing and Beat Model (Parity-Critical)

## 5.1 Authoritative Beat Definitions

The chart is authored in beats, while DSP frames and playback run in time. The TV MUST convert between time and beats deterministically using the rules below.

Definitions:
- `GAPms`: the integer value of `#GAP:` in milliseconds.
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
  - For duet songs, compute the next-line start time as the earlier of the two tracks.
  - The seek target MUST be clamped to at least `startSec`.



# 6. Scoring (Parity-Critical)

## 6.1 Scoring Overview

 Beat-based scoring, normalized to 10000 total. Line bonus ON reserves 1000 for line bonus and distributes remaining points via note value normalization.

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

- TV shows QR code and a short join code representing the current session endpoint (Section 8.1).
- TV shows a roster of connected device names (up to **10**).
- Join admission (normative):
 - Phones MAY join while the session is **Open** until the roster reaches 10 devices.
 - Additional phones MUST be rejected with an `error` (e.g., `code="session_full"`).
- During **Locked** state, new joins MUST be rejected with an `error` (e.g., `code="session_locked"`).

**Roster actions (normative)**
- **Rename device**: changes the display label shown on TV and stored by `clientId` for future use within the same session.
- **Kick device**: disconnects the device immediately; the roster entry is removed.
- **Forget device**: removes the stored display label for that `clientId` and disconnects the device; a future join is treated as a fresh device with default name.
- Kick/Forget MUST use a confirm dialog with default focus on Cancel.

**Wireframe (TV pairing info + roster management; spec-only interactions)**
```text
+--------------------------------------------------------------------------------+
| PAIR / JOIN                                                                    |
+--------------------------------------------------------------------------------+
| Join this session:                                                             |
|   [   QR CODE   ]             Code: ABCD                                       |
|                                                                                |
| Connected devices (up to 10):                                                  |
|  > Pixel-7        Connected                                                    |
|    iPhone-13      Connected                                                    |
|    ...                                                                         |
|                                                                                |
| Actions on selected device:  [Rename] [Kick] [Forget]                         |
+--------------------------------------------------------------------------------+
| Hints: OK=Select/Action   Back=Return                                          |
+--------------------------------------------------------------------------------+
```

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
- `ping` fields: `nonce`, `tSendTvMs` (TV time) or `tSendPhoneMs` (phone time) depending on sender 
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

- Ping/pong: every **2s** per phone; use median of last 5 RTTs for offset smoothing.

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

# 11. Parity Test Suite

## 11.1 Golden Parsing Fixtures

 Create fixture pack: solo basic, duet overlap + swap, rap, variable BPM, video+videogap, instrumental.

## 11.2 Golden Scoring Fixtures

 Beat-indexed test streams (toneValid/midiNote) with expected Notes/Golden/Line/Total outputs (exact).

## 11.3 Live Network Tests

 Jitter/loss/disconnect injection tests + acceptance thresholds.

## 11.4 Test Report Format

 Define required outputs: test_report.md + diffs + logs; PASS criteria gate.

# Appendix A: Supported Tags Reference

 Complete table of tags, units, defaults, and gameplay impact.

# Appendix B: Protocol Schemas

 JSON schemas for all messages.

# Appendix C: Fixture Inventory

Appendix C contains normative input fixtures and their expected, deterministic receiver-side reconstruction results.

The fixtures in this appendix are NOT end-to-end scoring fixtures. They validate protocol parsing, ordering, timestamp handling, and the semantics of toneValid/midiNote.

General rules (applies to all Appendix C fixtures):
- The receiver MUST be able to parse all messages and ignore unknown fields.
- The receiver MUST reconstruct an ordered frame stream using frame.seq and/or frame.ts (see Section 9), independent of message arrival time.
- The receiver MUST NOT interpret any specific `midiNote` value (including 0) as silence. Silence/unvoiced is represented by `toneValid=false`.

## C.1 gangnamstyle-normal-5s (protocol + ordered frames)

Input file: appendixC_gangnamstyle_normal_5s_stream.json

Expected result (receiver reconstruction):
```json
{
  "batching": {
    "batch_count": 50,
    "batch_interval_ms": 100,
    "frames_per_batch": 5
  },
  "covers_capture_time_ms": {
    "duration": 4980,
    "end": 4980,
    "start": 0
  },
  "expected_receiver_reconstruction": {
    "fps": 50.0,
    "frame_count": 250,
    "missing_midiNote": 0,
    "seq": {
      "max": 249,
      "min": 0,
      "must_be_contiguous": true
    },
    "midiNote_counts": {
      "0": 25,
      "1": 25,
      "2": 25,
      "3": 25,
      "4": 25,
      "5": 25,
      "6": 25,
      "7": 25,
      "8": 25,
      "9": 25
    },
    "toneValid_counts": {
      "true": 250
    },
    "ts_ms": {
      "max": 4980,
      "min": 0,
      "step": 20
    }
  },
  "fixture_file": "appendixC_gangnamstyle_normal_5s_stream.json",
  "notes": [
    "No specific midiNote value implies silence. Silence/unvoiced MUST be represented by toneValid=false.",
    "This fixture is synthetic and is used to validate protocol parsing, ordering, and basic pitch frame handling (not musical correctness)."
  ]
}
```

## C.2 gangnamstyle-rap-5s (unvoiced frames + large batches)

Input file: appendixC_gangnamstyle_rap_5s_stream.json

Expected result (receiver reconstruction):
```json
{
  "batching": {
    "batch_count": 10,
    "batch_interval_ms": 500,
    "frames_per_batch": 25
  },
  "covers_capture_time_ms": {
    "duration": 4980,
    "end": 4980,
    "start": 0
  },
  "expected_receiver_reconstruction": {
    "fps": 50.0,
    "frame_count": 250,
    "missing_midiNote": 65,
    "seq": {
      "max": 249,
      "min": 0,
      "must_be_contiguous": true
    },
    "midiNote_counts_for_toneValid_true": {
      "0": 13,
      "1": 18,
      "2": 14,
      "3": 17,
      "4": 14,
      "5": 16,
      "6": 14,
      "7": 17,
      "8": 14,
      "9": 17,
      "10": 14,
      "11": 17
    },
    "toneValid_counts": {
      "false": 65,
      "true": 185
    },
    "ts_ms": {
      "max": 4980,
      "min": 0,
      "step": 20
    }
  },
  "fixture_file": "appendixC_gangnamstyle_rap_5s_stream.json",
  "notes": [
    "For frames where toneValid=false, midiNote is omitted. The receiver MUST treat those frames as unvoiced/silence for scoring and UI.",
    "This fixture validates handling of intermittent unvoiced frames and larger batch sizes."
  ]
}
```

## C.3 Future scoring fixtures (reserved)

Scoring fixtures (song + pitch stream + expected total score breakdown) are out of scope for Appendix C in v0.3. When added, each scoring fixture MUST provide:
- Covered song_time_ms window and all timing assumptions (BPM, GAP, micDelayMs, and any drift correction settings).
- Expected per-checkpoint state (active note id, judgement bucket) and final score totals.
- Enough detail to reproduce results deterministically without referencing USDX source code.

# Appendix D. Fixture-driven UATs (planned, deterministic descriptions)

This appendix lists mandatory MVP test cases that SHOULD be backed by fixtures. The fixtures themselves are NOT included here; this section defines deterministic inputs and expected outcomes so they can be generated later.

Conventions:
- A 'song fixture' is a self-contained directory containing a `.txt` chart and referenced media placeholders (audio file may be a silent stub).
- A 'pitch fixture' is a JSON message log representing `pitchFrame`/`pitchBatch` traffic (Section 8.3) aligned to a song_time_ms window.
- 'Expected outcome' MUST be assertable by an automated test without human interpretation.

## D.1 Recursive song discovery
Purpose: verify recursive `.txt` discovery across nested folders and stable sorting.
Inputs: create a songs root with nested subfolders containing 3 `.txt` files at different depths. Each `.txt` has distinct #ARTIST/#ALBUM/#TITLE values.
Expected outcome: library index contains exactly 3 entries; sort order is Artist, then Album, then Title (Section 3.4).

## D.2 Reject missing required header tag
Purpose: verify rejection and diagnostic collection.
Inputs: `.txt` missing `#BPM:` (or empty `#TITLE:`). Provide line numbers deterministically by placing the missing/empty tag at a known line.
Expected outcome: song is rejected (Section 3.2) with an error diagnostic that includes (a) 1-based line number and (b) reason code indicating which required field is missing.

## D.3 Reject missing required audio file
Purpose: verify required audio existence check.
Inputs: `.txt` with required headers, but `#AUDIO:` points to a non-existent filename in the same directory.
Expected outcome: song is rejected with a diagnostic indicating missing audio file (Section 3.2).

## D.4 Unknown header tag is logged but parsing continues
Purpose: verify unknown tags are preserved and warned.
Inputs: `.txt` includes `#FOO:bar` before notes; required tags present.
Expected outcome: song is accepted; diagnostics include a warning for unknown tag; `CustomTags` contains `(FOO, bar)` in the original order (Section 4.3).

## D.5 Recoverable body grammar issue is logged but parsing continues
Purpose: verify auto-fix and non-fatal handling.
Inputs: `.txt` body contains an empty line, whitespace-only line, and a line with an unknown leading token `X ...`.
Expected outcome: song is accepted; diagnostics include warnings for the unknown/invalid line(s); notes parsing continues; line count and note count match the parsable lines (Section 4.3).

## D.6 Version-specific tag handling
Purpose: verify `#NOTESGAP`/`#RESOLUTION` ignored for version >=1.0.0 and that `#RELATIVE` is rejected for version >=1.0.0.
Inputs: two `.txt` files identical except #VERSION and presence of legacy tags.
Expected outcome:
- For #VERSION >= 1.0.0: `#NOTESGAP` and `#RESOLUTION` are ignored with info diagnostics; `#RELATIVE` causes rejection (Section 4.2).
- For #VERSION absent or <1.0.0: legacy tags are honored (where specified) and song can be accepted.

## D.7 Protocol frame ordering and batching
Purpose: verify seq monotonicity and batch parsing.
Inputs: a pitch fixture containing (a) a `pitchBatch` with 5 frames, then (b) a single `pitchFrame` with a lower `seq`.
Expected outcome: batched frames are accepted in order; the regressing `seq` frame is dropped and produces a warning diagnostic (Section 8.3).

## D.8 midiNote octave normalization scoring
Purpose: verify octave wrapping logic.
Inputs: song fixture with one Normal note at target tone T for a short duration. Pitch fixture where phone sends `midiNote` whose derived `toneUsdx = midiNote - 36` is an octave away (T +/- 12) while `toneValid=true`.
Expected outcome: detection beats during the note score as hits as if the singer were in the closest octave (Section 6.4).

## D.9 Rap presence-only scoring gate
Purpose: verify rap notes ignore pitch difference but require voicing.
Inputs: song fixture containing a Rap note spanning multiple detection beats. Pitch fixture with alternating `toneValid=false` and `toneValid=true` frames.
Expected outcome: only detection beats whose selected frame has `toneValid=true` count as hits; pitch value does not affect hit/miss (Section 6.2).

## D.10 Auto mic delay adjustment
Purpose: verify TV-side drift/latency compensation algorithm.
Inputs: a pitch fixture with consistent positive lateness bias (arrivalTimeTv later than mapped capture time) for >10s, stable median >80ms.
Expected outcome: TV increases `effectiveMicDelayMs` in +10ms steps no more than once per 10s window until median bias is within +/-80ms, respecting [0,400] clamp and reset conditions (Section 9.2).
