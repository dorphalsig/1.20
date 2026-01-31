# F03 — Body grammar: token recognition + invalidation rules

This acceptance fixture covers the F03 requirements from Appendix F.5.

It contains two parts:

1) A **discovery/index fixture** (Appendix F.4.3) under `songs_root/`.
   - A harness should recursively discover `song.txt` files under `songs_root/`.
   - It should load/validate each song according to Sections 4.2–4.3.
   - It should compare deterministic results to `expected.discovery.json`.

2) A **scoring subcase** (Appendix F.4.2) under `scoring/freestyle_only/`.
   - This subcase exists solely to assert the rule from Section 6.2:
     - Freestyle notes (`F`) are excluded from hit detection and contribute **0 points**.

## `expected.discovery.json` (repo convention)

This file follows the `expected.discovery.json` convention from Appendix F.4.3.

For valid songs, this fixture asserts a minimal deterministic body summary:
- `bodySummary.track0.noteTypeCounts`

## `scoring/freestyle_only/` (repo convention within this fixture)

This directory is a self-contained scoring fixture:
- `song.txt`
- `expected.parsedSong.json` (Appendix D)
- `pitchFrames.jsonl` (Appendix D.3)
- `expected.score.json`

`expected.score.json` asserts only the deterministic outcome needed for this subcase:
- per-beat `scoreDelta == 0` while the active note is `Freestyle`
- `scoreTotalInt == 0`

Note: this subcase keeps the chart intentionally minimal and does not attempt to cover normalization/line-bonus math.
