# pyin_rs Flutter + Rust (FRB)
## Paths
flutter is installed in /opt/flutter, make sure to add /opt/flutter/bin to the PATH environment var.

## CI gate commands

Rust + headless Flutter tests:

```bash
cargo test
flutter pub get
flutter test test/pyin_frb_stream_test.dart
```

`cargo test` generates the PCM fixtures in `integration_test/assets/pcm/` when
missing, so the headless Flutter test can load the assets.

## Linux integration test (desktop)

We run Linux integration tests via `flutter drive` rather than `flutter test -d linux`:

```bash
tool/run_integration_linux.sh
```

The script builds the Linux bundle and runs the integration test with
`xvfb-run -a` when no display is available.
