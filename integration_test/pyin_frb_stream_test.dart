import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:pyin_rs/pyin_frb_wrapper.dart';

const _chunkPattern = [511, 1023, 2048, 333, 4097, 777, 1500];

Map<int, int> _counts(Iterable<int> values) {
  final map = <int, int>{};
  for (final v in values) {
    map[v] = (map[v] ?? 0) + 1;
  }
  return map;
}

int _mode(List<int> values) {
  final c = _counts(values);
  return c.entries.reduce((a, b) => a.value >= b.value ? a : b).key;
}

Future<List<int>> _streamFixture(String assetPath) async {
  final data = await rootBundle.load(assetPath);
  final bytes = data.buffer.asUint8List();
  final out = <int>[];
  final proc = await PyinFrbStreamProcessor.create(
    sampleRateHz: 48000,
    windowMs: 43,
    hopMs: 5,
    onNote: (note) {
      if (note != null) out.add(note);
    },
  );
  var offset = 0;
  var i = 0;
  while (offset < bytes.length) {
    final size = _chunkPattern[i % _chunkPattern.length];
    final end = (offset + size).clamp(0, bytes.length);
    final chunk = Uint8List.sublistView(bytes, offset, end);
    proc.push(chunk);
    offset = end;
    i += 1;
  }
  await _waitForNotes(out, 10);
  await proc.dispose();
  return out;
}

Future<void> _waitForNotes(List<int> notes, int minCount) async {
  final deadline = DateTime.now().add(const Duration(seconds: 2));
  while (notes.length < minCount && DateTime.now().isBefore(deadline)) {
    await Future<void>.delayed(const Duration(milliseconds: 10));
  }
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('push triggers onNote callback', (_) async {
    final data = await rootBundle.load(
      'integration_test/assets/pcm/a4_440_pcm16le_mono.pcm',
    );
    final bytes = data.buffer.asUint8List();
    final out = <int>[];
    final proc = await PyinFrbStreamProcessor.create(
      sampleRateHz: 48000,
      windowMs: 43,
      hopMs: 5,
      onNote: (note) {
        if (note != null) out.add(note);
      },
    );

    proc.push(bytes.sublist(0, 2048));
    await _waitForNotes(out, 1);
    await proc.dispose();
    expect(out, isNotEmpty);
  });

  final tests = {
    'integration_test/assets/pcm/a3_220_pcm16le_mono.pcm': 57,
    'integration_test/assets/pcm/a4_440_pcm16le_mono.pcm': 69,
    'integration_test/assets/pcm/c6_1046_50_pcm16le_mono.pcm': 84,
    'integration_test/assets/pcm/c2_pcm16le_mono.pcm': 36,
  };

  tests.forEach((asset, expected) {
    testWidgets(asset, (_) async {
      final voiced = await _streamFixture(asset);
      expect(voiced.length, greaterThanOrEqualTo(10));
      final warmed = voiced.skip(3).toList();
      expect((_mode(warmed) - expected).abs(), lessThanOrEqualTo(1));
    });
  });
}
