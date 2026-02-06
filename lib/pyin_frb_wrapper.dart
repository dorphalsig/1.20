import 'dart:async';
import 'dart:typed_data';

// Adjust import path/package to your generated FRB bindings.
import 'package:pyin_rs/frb_generated.dart';

class PyinFrbStreamProcessor {
  final PyinProcessor _processor;
  final StreamController<int> _controller;
  final StreamSubscription<int> _subscription;

  PyinFrbStreamProcessor._(
    this._processor,
    this._controller,
    this._subscription,
  );

  static Future<PyinFrbStreamProcessor> create({
    required int sampleRateHz,
    required int windowMs,
    required int hopMs,
    required void Function(int? note) onNote,
  }) async {
    await initLogging();
    final proc = await newProcessor(
      sampleRateHz: sampleRateHz,
      windowMs: windowMs,
      hopMs: hopMs,
    );
    final controller = StreamController<int>();
    await startEventStream(proc: proc, sink: controller.sink);
    final subscription = controller.stream.listen((note) {
      onNote(note == 255 ? null : note);
    });
    return PyinFrbStreamProcessor._(proc, controller, subscription);
  }

  void push(Uint8List bytes) {
    unawaited(pushPcmTask(proc: _processor, pcm16leBytes: bytes));
  }

  Future<void> dispose() async {
    await _subscription.cancel();
    await _controller.close();
  }
}
