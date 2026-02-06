#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/flutter/bin:${PATH}"

cargo run --bin generate_pcm_fixtures
flutter pub get

if ! pkg-config --exists gtk+-3.0; then
  echo "gtk+-3.0 not found; installing Linux desktop deps." >&2
  apt-get update
  apt-get install -y clang cmake ninja-build pkg-config libgtk-3-dev xvfb
fi

flutter build linux --debug
cmake --install build/linux/x64/debug
ls -la build/linux/x64/debug/bundle/

if [[ -z "${DISPLAY:-}" ]]; then
  echo "DISPLAY not set; running integration test with xvfb-run." >&2
  xvfb-run -a flutter drive -d linux \
    --driver test_driver/integration_test.dart \
    --target integration_test/pyin_frb_stream_test.dart
else
  flutter drive -d linux \
    --driver test_driver/integration_test.dart \
    --target integration_test/pyin_frb_stream_test.dart
fi
