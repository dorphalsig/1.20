import 'dart:typed_data';

// Adjust import path/package to your generated FRB bindings.
import 'package:pyin_rs/frb_generated.dart';

class PyinFrbStreamProcessor {
  final PyinProcessor _processor;

  PyinFrbStreamProcessor._(this._processor);

  static Future<PyinFrbStreamProcessor> create({
    required int sampleRateHz,
    required int windowMs,
    required int hopMs,
  }) async {
    await initLogging();
    final proc = await newProcessor(
      sampleRateHz: sampleRateHz,
      windowMs: windowMs,
      hopMs: hopMs,
    );
    return PyinFrbStreamProcessor._(proc);
  }

  Future<int?> push(Uint8List bytes) async {
    final midi = await pushAndGetMidi(proc: _processor, pcm16leBytes: bytes);
    return midi == 255 ? null : midi;
  }
}
