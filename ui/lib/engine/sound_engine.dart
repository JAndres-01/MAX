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
        // Fast spawn without waiting for completion
        if (File('/usr/bin/pw-play').existsSync()) {
          Process.start('/usr/bin/pw-play', [wavPath], mode: ProcessStartMode.detached);
        } else if (File('/usr/bin/paplay').existsSync()) {
          Process.start('/usr/bin/paplay', [wavPath], mode: ProcessStartMode.detached);
        } else if (File('/usr/bin/aplay').existsSync()) {
          Process.start('/usr/bin/aplay', ['-q', wavPath], mode: ProcessStartMode.detached);
        }
      } else if (Platform.isWindows) {
        // Windows Media SoundPlayer via PowerShell async background
        Process.start(
          'powershell',
          ['-c', "(New-Object Media.SoundPlayer '$wavPath').PlaySync()"],
          mode: ProcessStartMode.detached,
        );
      }
    } catch (e) {
      debugPrint('SoundEngine notice: $e');
    }
  }
}
