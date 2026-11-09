import 'dart:async';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as p;
import 'max_engine_bridge.dart';

class SoundEngine {
  static String? _cachedSoundsDir;

  static String get soundsDir {
    if (_cachedSoundsDir != null) return _cachedSoundsDir!;
    final home = Platform.environment['HOME'] ??
        Platform.environment['USERPROFILE'] ??
        '/tmp';
    _cachedSoundsDir = p.join(home, '.config', 'max-drop', 'sounds');
    return _cachedSoundsDir!;
  }

  static void playSnap({MaxEngineBridge? bridge}) {
    _playSound('snap.wav', bridge: bridge);
  }

  static void playClick({MaxEngineBridge? bridge}) {
    _playSound('click.wav', bridge: bridge);
  }

  static void playComplete({MaxEngineBridge? bridge}) {
    _playSound('complete.wav', bridge: bridge);
  }

  static void _playSound(String filename, {MaxEngineBridge? bridge}) {
    if (bridge != null && bridge.currentConfig != null) {
      if (!bridge.currentConfig!.soundEffects) {
        return;
      }
    }

    try {
      final wavPath = p.join(soundsDir, filename);
      final wavFile = File(wavPath);
      if (!wavFile.existsSync()) return;

      if (Platform.isLinux) {
        // Fast asynchronous execution with automated reaping to prevent defunct zombie processes
        if (File('/usr/bin/pw-play').existsSync()) {
          unawaited(Process.run('/usr/bin/pw-play', [wavPath]));
        } else if (File('/usr/bin/paplay').existsSync()) {
          unawaited(Process.run('/usr/bin/paplay', [wavPath]));
        } else if (File('/usr/bin/aplay').existsSync()) {
          unawaited(Process.run('/usr/bin/aplay', ['-q', wavPath]));
        }
      } else if (Platform.isWindows) {
        // Windows Media SoundPlayer via PowerShell async background
        unawaited(Process.run(
          'powershell',
          ['-NoProfile', '-NonInteractive', '-c', "(New-Object Media.SoundPlayer '$wavPath').PlaySync()"],
        ));
      }
    } catch (e) {
      debugPrint('SoundEngine notice: $e');
    }
  }
}
