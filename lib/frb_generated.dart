import 'dart:math' as math;
import 'dart:typed_data';

// Test shim for FRB-generated API surface.
// In production this file is replaced by flutter_rust_bridge generated bindings.
class PyinProcessor {
  PyinProcessor({
    required this.sampleRateHz,
    required this.windowMs,
    required this.hopMs,
  })  : frameSize = math.max(1, (sampleRateHz * windowMs / 1000).round()),
        hopSize = math.max(1, (sampleRateHz * hopMs / 1000).round());

  final int sampleRateHz;
  final int windowMs;
  final int hopMs;
  final int frameSize;
  final int hopSize;

  final List<int> _samples = <int>[];
  int? _carry;
}

Future<void> initLogging() async {}

Future<PyinProcessor> newProcessor({
  required int sampleRateHz,
  required int windowMs,
  required int hopMs,
}) async {
  return PyinProcessor(
    sampleRateHz: sampleRateHz,
    windowMs: windowMs,
    hopMs: hopMs,
  );
}

Future<int> pushAndGetMidi({
  required PyinProcessor proc,
  required Uint8List pcm16leBytes,
}) async {
  _append(proc, pcm16leBytes);
  if (proc.frameSize < proc.hopSize) return 255;
  var latest = 255;
  while (proc._samples.length >= proc.frameSize) {
    final frame = proc._samples.sublist(0, proc.frameSize);
    final hz = _hz(frame, proc.sampleRateHz);
    if (hz != null) {
      latest = (69 + 12 * (math.log(hz / 440.0) / math.ln2)).round().clamp(0, 127);
    }
    final drop = math.min(proc.hopSize, proc._samples.length);
    proc._samples.removeRange(0, drop);
  }
  return latest;
}

void _append(PyinProcessor proc, Uint8List bytes) {
  var i = 0;
  if (proc._carry != null) {
    if (bytes.isEmpty) return;
    proc._samples.add(_i16(proc._carry!, bytes[0]));
    proc._carry = null;
    i = 1;
  }
  while (i + 1 < bytes.length) {
    proc._samples.add(_i16(bytes[i], bytes[i + 1]));
    i += 2;
  }
  if (i < bytes.length) {
    proc._carry = bytes[i];
  }
}

int _i16(int lo, int hi) {
  final v = (hi << 8) | lo;
  return v >= 0x8000 ? v - 0x10000 : v;
}

double? _hz(List<int> frame, int sampleRateHz) {
  var crossings = 0;
  for (var i = 1; i < frame.length; i++) {
    final a = frame[i - 1];
    final b = frame[i];
    if ((a <= 0 && b > 0) || (a >= 0 && b < 0)) {
      crossings++;
    }
  }
  if (crossings < 4) return null;
  final hz = (crossings * sampleRateHz) / (2.0 * frame.length);
  if (hz < 40 || hz > 2000) return null;
  return hz;
}
