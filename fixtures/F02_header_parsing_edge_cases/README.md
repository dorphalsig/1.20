# F02 — Header parsing edge cases

This acceptance fixture is a **discovery/index fixture** (Appendix F.4.3): it contains a small `songs_root/` tree with multiple single-song subcases.

A harness should:
1. Recursively discover `song.txt` files under `songs_root/`.
2. Attempt to load/validate each song according to Sections 4.2–4.3.
3. Compare the deterministic results to `expected.discovery.json`.

## `expected.discovery.json` (repo convention)

This file follows the `expected.discovery.json` convention from Appendix F.4.3.

In addition to the required fields (`isValid`, `invalidReasonCode`, `invalidLineNumber`), this fixture asserts a small set of deterministic header outcomes for **valid** songs:

- `header.title`
- `header.artist`
- `header.version`
- `header.bpmFile`
- `header.audioResolved`
- `derived.previewStartSec`
- `header.customTagsOrdered` (ordered list of `{name,value}` pairs, matching Section 4.3 CustomTags representation)

Invalid songs omit `header`/`derived` fields and only assert validity + diagnostics.
