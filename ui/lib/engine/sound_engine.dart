import 'dart:async';
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:path/path.dart' as p;
import 'max_engine_bridge.dart';

class SoundEngine {
  static const MethodChannel _windowChannel = MethodChannel('max/window');
  static String? _cachedSoundsDir;

  static String get soundsDir {
    if (_cachedSoundsDir != null) return _cachedSoundsDir!;
    final home = Platform.environment['HOME'] ??
        Platform.environment['USERPROFILE'] ??
        '/tmp';
    _cachedSoundsDir = p.join(home, '.config', 'max-drop', 'sounds');
    return _cachedSoundsDir!;
  }

  static String? _resolveSoundPath(String filename) {
    final primary = p.join(soundsDir, filename);
    if (File(primary).existsSync()) {
      return primary;
    }
    if (Platform.isWindows) {
      final appData = Platform.environment['APPDATA'];
      if (appData != null && appData.isNotEmpty) {
        final fallback = p.join(appData, 'max', 'max-drop', 'config', 'sounds', filename);
        if (File(fallback).existsSync()) {
          return fallback;
        }
      }
    }
    return null;
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
      final wavPath = _resolveSoundPath(filename);
      if (wavPath == null) return;

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
        unawaited(
          _windowChannel
              .invokeMethod('playSound', {'path': wavPath})
              .catchError((_) {}),
        );
      }
    } catch (e) {
      debugPrint('SoundEngine notice: $e');
    }
  }
}
