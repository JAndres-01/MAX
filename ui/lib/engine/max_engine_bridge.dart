import 'dart:async';
import 'dart:convert';
import 'dart:ffi' as ffi;
import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'package:path/path.dart' as p;
import 'package:web_socket_channel/web_socket_channel.dart';

typedef MaxCoreStartNative = ffi.Int32 Function();
typedef MaxCoreStartDart = int Function();

class PeerDevice {
  final String ip;
  final int port;
  final String alias;
  final String? deviceModel;
  final String deviceType;
  final String fingerprint;
  final String protocol;

  PeerDevice({
    required this.ip,
    required this.port,
    required this.alias,
    this.deviceModel,
    required this.deviceType,
    required this.fingerprint,
    required this.protocol,
  });

  factory PeerDevice.fromJson(Map<String, dynamic> json) {
    return PeerDevice(
      ip: json['ip'] ?? '',
      port: json['port'] ?? 53317,
      alias: json['alias'] ?? 'Dispositivo',
      deviceModel: json['deviceModel'],
      deviceType: (json['deviceType'] ?? 'mobile').toString().toLowerCase(),
      fingerprint: json['fingerprint'] ?? '',
      protocol: json['protocol'] ?? 'https',
    );
  }
}

class IncomingFileDto {
  final String id;
  final String fileName;
  final int size;
  final String fileType;

  IncomingFileDto({
    required this.id,
    required this.fileName,
    required this.size,
    required this.fileType,
  });

  factory IncomingFileDto.fromJson(Map<String, dynamic> json) {
    return IncomingFileDto(
      id: json['id'] ?? '',
      fileName: json['fileName'] ?? 'archivo',
      size: json['size'] ?? 0,
      fileType: json['fileType'] ?? 'application/octet-stream',
    );
  }
}

class IncomingRequestEvent {
  final String sessionId;
  final String senderAlias;
  final String senderDeviceType;
  final String senderFingerprint;
  final List<IncomingFileDto> files;
  final int totalSize;

  IncomingRequestEvent({
    required this.sessionId,
    required this.senderAlias,
    required this.senderDeviceType,
    required this.senderFingerprint,
    required this.files,
    required this.totalSize,
  });

  factory IncomingRequestEvent.fromJson(Map<String, dynamic> json) {
    final sender = json['sender'] as Map<String, dynamic>? ?? {};
    final filesList = (json['files'] as List<dynamic>? ?? [])
        .map((e) => IncomingFileDto.fromJson(e as Map<String, dynamic>))
        .toList();
    return IncomingRequestEvent(
      sessionId: json['sessionId'] ?? '',
      senderAlias: sender['alias'] ?? 'Dispositivo',
      senderDeviceType: (sender['deviceType'] ?? 'mobile').toString().toLowerCase(),
      senderFingerprint: sender['fingerprint'] ?? '',
      files: filesList,
      totalSize: json['totalSize'] ?? 0,
    );
  }
}

class TransferProgressEvent {
  final String sessionId;
  final String fileName;
  final int bytesTransferred;
  final int totalBytes;
  final double speedMbps;
  final int filesCompleted;
  final int totalFiles;

  TransferProgressEvent({
    required this.sessionId,
    required this.fileName,
    required this.bytesTransferred,
    required this.totalBytes,
    required this.speedMbps,
    this.filesCompleted = 0,
    this.totalFiles = 1,
  });

  double get fraction =>
      totalBytes > 0 ? (bytesTransferred / totalBytes).clamp(0.0, 1.0) : 0.0;

  factory TransferProgressEvent.fromJson(Map<String, dynamic> json) {
    final rawBps = (json['speedBytesPerSec'] as num?)?.toDouble() ?? 0.0;
    final mbps = (json['speedMbps'] as num?)?.toDouble() ?? (rawBps / 1048576.0);
    return TransferProgressEvent(
      sessionId: json['sessionId'] ?? '',
      fileName: json['fileName'] ?? '',
      bytesTransferred: json['bytesTransferred'] ?? 0,
      totalBytes: json['totalBytes'] ?? 1,
      speedMbps: mbps,
      filesCompleted:
          (json['filesCompleted'] ?? json['files_completed'] as num?)?.toInt() ??
              0,
      totalFiles:
          (json['totalFiles'] ?? json['total_files'] as num?)?.toInt() ?? 1,
    );
  }
}

class TransferCompleteEvent {
  final String sessionId;
  final bool success;
  final String message;
  final String? filePath;
  final String? folderPath;
  final bool isIncoming;
  final int fileCount;
  final int totalFileCount;
  final bool isPartial;

  TransferCompleteEvent({
    required this.sessionId,
    required this.success,
    required this.message,
    this.filePath,
    this.folderPath,
    this.isIncoming = false,
    this.fileCount = 1,
    this.totalFileCount = 1,
    this.isPartial = false,
  });

  factory TransferCompleteEvent.fromJson(Map<String, dynamic> json) {
    final fileCountVal =
        (json['file_count'] ?? json['fileCount'] as num?)?.toInt() ?? 1;
    final totalCountVal =
        (json['total_file_count'] ?? json['totalFileCount'] as num?)?.toInt() ??
            fileCountVal;
    return TransferCompleteEvent(
      sessionId: json['session_id'] ?? json['sessionId'] ?? '',
      success: json['success'] ?? true,
      message: json['message'] ?? '',
      filePath: json['file_path'] ?? json['filePath'],
      folderPath: json['folder_path'] ?? json['folderPath'],
      isIncoming: json['is_incoming'] ?? json['isIncoming'] ?? false,
      fileCount: fileCountVal,
      totalFileCount: totalCountVal,
      isPartial: json['is_partial'] ?? json['isPartial'] ?? false,
    );
  }
}

class TransferHistoryItemDto {
  final String id;
  final String fileName;
  final String? filePath;
  final int fileSize;
  final bool isIncoming;
  final String deviceAlias;
  final int timestamp;
  final bool success;

  TransferHistoryItemDto({
    required this.id,
    required this.fileName,
    this.filePath,
    required this.fileSize,
    required this.isIncoming,
    required this.deviceAlias,
    required this.timestamp,
    required this.success,
  });

  factory TransferHistoryItemDto.fromJson(Map<String, dynamic> json) {
    return TransferHistoryItemDto(
      id: json['id'] ?? '',
      fileName: json['fileName'] ?? json['file_name'] ?? 'archivo',
      filePath: json['filePath'] ?? json['file_path'],
      fileSize: (json['fileSize'] ?? json['file_size'] as num?)?.toInt() ?? 0,
      isIncoming: json['isIncoming'] ?? json['is_incoming'] ?? false,
      deviceAlias: json['deviceAlias'] ?? json['device_alias'] ?? 'Dispositivo',
      timestamp: (json['timestamp'] as num?)?.toInt() ?? 0,
      success: json['success'] ?? true,
    );
  }

  String get formattedSize {
    if (fileSize < 1024) return '$fileSize B';
    if (fileSize < 1024 * 1024) return '${(fileSize / 1024).toStringAsFixed(1)} KB';
    if (fileSize < 1024 * 1024 * 1024) {
      return '${(fileSize / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(fileSize / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  String get formattedDate {
    if (timestamp == 0) return '';
    final dt = DateTime.fromMillisecondsSinceEpoch(timestamp * 1000);
    final now = DateTime.now();
    final diff = now.difference(dt);
    if (diff.inMinutes < 1) return 'Ahora';
    if (diff.inMinutes < 60) return 'Hace ${diff.inMinutes}m';
    if (diff.inHours < 24) return 'Hace ${diff.inHours}h';
    return '${dt.day.toString().padLeft(2, '0')}/${dt.month.toString().padLeft(2, '0')} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}

class AppConfigModel {
  String alias;
  String deviceType;
  String deviceModel;
  int port;
  String protocol;
  String downloadDir;
  bool autoAccept;
  List<String> trustedDevices;
  String fingerprint;
  bool discoveryEnabled;
  int hoverDelayMs;
  int autoHideDelayMs;
  double notchIdleWidth;
  double panelOpacity;
  bool autostart;
  bool soundEffects;

  AppConfigModel({
    required this.alias,
    required this.deviceType,
    required this.deviceModel,
    required this.port,
    required this.protocol,
    required this.downloadDir,
    required this.autoAccept,
    required this.trustedDevices,
    required this.fingerprint,
    required this.discoveryEnabled,
    required this.hoverDelayMs,
    required this.autoHideDelayMs,
    required this.notchIdleWidth,
    required this.panelOpacity,
    required this.autostart,
    this.soundEffects = true,
  });

  factory AppConfigModel.fromJson(Map<String, dynamic> json) {
    return AppConfigModel(
      alias: json['alias'] ?? 'MAX',
      deviceType: json['deviceType'] ?? 'desktop',
      deviceModel: json['deviceModel'] ?? '',
      port: json['port'] ?? 53317,
      protocol: json['protocol'] ?? 'https',
      downloadDir: json['downloadDir'] ?? '',
      autoAccept: json['autoAccept'] ?? false,
      trustedDevices: List<String>.from(json['trustedDevices'] ?? []),
      fingerprint: json['fingerprint'] ?? '',
      discoveryEnabled: json['discoveryEnabled'] ?? true,
      hoverDelayMs: json['hoverDelayMs'] ?? 1000,
      autoHideDelayMs: json['autoHideDelayMs'] ?? 420,
      notchIdleWidth: (json['notchIdleWidth'] as num?)?.toDouble() ?? 144.0,
      panelOpacity: (json['panelOpacity'] as num?)?.toDouble() ?? 0.95,
      autostart: json['autostart'] ?? false,
      soundEffects: json['soundEffects'] ?? json['sound_effects'] ?? true,
    );
  }

  Map<String, dynamic> toJson() {
    return {
      'alias': alias,
      'deviceType': deviceType,
      'deviceModel': deviceModel,
      'port': port,
      'protocol': protocol,
      'downloadDir': downloadDir,
      'autoAccept': autoAccept,
      'trustedDevices': trustedDevices,
      'fingerprint': fingerprint,
      'discoveryEnabled': discoveryEnabled,
      'hoverDelayMs': hoverDelayMs,
      'autoHideDelayMs': autoHideDelayMs,
      'notchIdleWidth': notchIdleWidth,
      'panelOpacity': panelOpacity,
      'autostart': autostart,
      'soundEffects': soundEffects,
    };
  }

  AppConfigModel clone() {
    return AppConfigModel.fromJson(toJson());
  }
}

class MaxEngineBridge extends ChangeNotifier {
  static const String _baseUrl = 'http://127.0.0.1:53320';
  static const String _wsUrl = 'ws://127.0.0.1:53320/api/max/v1/events';

  List<PeerDevice> peers = [];
  IncomingRequestEvent? activeIncomingRequest;
  TransferProgressEvent? currentProgress;
  TransferCompleteEvent? lastCompletion;
  AppConfigModel? currentConfig;
  List<TransferHistoryItemDto> history = [];

  WebSocketChannel? _wsChannel;
  Timer? _reconnectTimer;
  Timer? _pollTimer;

  void initialize({bool isSettingsMode = false}) {
    if (!isSettingsMode) {
      _startEmbeddedRustCore();
      _pollTimer = Timer.periodic(const Duration(seconds: 2), (_) => fetchPeers());
    }
    _connectEventStream();
    fetchConfig();
    fetchHistory();
  }

  void _startEmbeddedRustCore() {
    try {
      final execDir = File(Platform.resolvedExecutable).parent.path;
      final candidates = Platform.isWindows
          ? [
              p.join(execDir, 'max_core.dll'),
              p.join(Directory.current.path, '..', 'core', 'target', 'release', 'max_core.dll'),
            ]
          : [
              p.join(execDir, 'lib', 'libmax_core.so'),
              p.join(execDir, 'libmax_core.so'),
              p.join(Directory.current.path, '..', 'core', 'target', 'release', 'libmax_core.so'),
              '/home/hyprand/Proyectos/antigravity/MAX/core/target/release/libmax_core.so',
            ];

      for (final path in candidates) {
        if (File(path).existsSync()) {
          final dylib = ffi.DynamicLibrary.open(path);
          final startFn = dylib
              .lookup<ffi.NativeFunction<MaxCoreStartNative>>('max_core_start')
              .asFunction<MaxCoreStartDart>();
          startFn();
          debugPrint('MAX Embedded Rust Engine initialized from: $path');
          return;
        }
      }
    } catch (e) {
      debugPrint('Notice loading FFI library: $e');
    }
  }

  Future<void> _connectEventStream() async {
    try {
      await _wsChannel?.sink.close();
      final channel = WebSocketChannel.connect(Uri.parse(_wsUrl));
      await channel.ready;
      _wsChannel = channel;
      channel.stream.listen(
        (message) {
          if (message is String) {
            _handleEventJson(message);
          }
        },
        onDone: _scheduleReconnect,
        onError: (_) => _scheduleReconnect(),
      );
    } catch (_) {
      _scheduleReconnect();
    }
  }

  void _scheduleReconnect() {
    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(const Duration(seconds: 2), _connectEventStream);
  }

  void _handleEventJson(String raw) {
    try {
      final decoded = jsonDecode(raw) as Map<String, dynamic>;
      final type = decoded['type'] as String?;
      final data = decoded['data'];

      if (type == 'peers_updated' && data is List) {
        peers = data
            .map((e) => PeerDevice.fromJson(e as Map<String, dynamic>))
            .toList();
        notifyListeners();
      } else if (type == 'incoming_request' && data is Map<String, dynamic>) {
        activeIncomingRequest = IncomingRequestEvent.fromJson(data);
        notifyListeners();
      } else if (type == 'transfer_progress' && data is Map<String, dynamic>) {
        currentProgress = TransferProgressEvent.fromJson(data);
        notifyListeners();
      } else if (type == 'transfer_completed' && data is Map<String, dynamic>) {
        lastCompletion = TransferCompleteEvent.fromJson(data);
        fetchHistory();
        notifyListeners();
      } else if (type == 'history_updated' && data is List) {
        history = data
            .map((e) => TransferHistoryItemDto.fromJson(e as Map<String, dynamic>))
            .toList();
        notifyListeners();
      } else if (type == 'config_updated' && data is Map<String, dynamic>) {
        currentConfig = AppConfigModel.fromJson(data);
        notifyListeners();
      }
    } catch (_) {}
  }

  Future<List<TransferHistoryItemDto>> fetchHistory() async {
    try {
      final resp = await http
          .get(Uri.parse('$_baseUrl/api/max/v1/history'))
          .timeout(const Duration(milliseconds: 900));
      if (resp.statusCode == 200) {
        final List<dynamic> list = jsonDecode(resp.body);
        history = list
            .map((e) => TransferHistoryItemDto.fromJson(e as Map<String, dynamic>))
            .toList();
        notifyListeners();
        return history;
      }
    } catch (_) {}
    return history;
  }

  Future<bool> clearHistory() async {
    try {
      final resp = await http
          .post(Uri.parse('$_baseUrl/api/max/v1/history/clear'))
          .timeout(const Duration(milliseconds: 900));
      if (resp.statusCode == 200) {
        history.clear();
        notifyListeners();
        return true;
      }
    } catch (_) {}
    return false;
  }

  Future<bool> openPath(String targetPath, {bool select = false}) async {
    try {
      final resp = await http.post(
        Uri.parse('$_baseUrl/api/max/v1/open'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'path': targetPath, 'select': select}),
      ).timeout(const Duration(milliseconds: 900));
      if (resp.statusCode == 200) {
        return true;
      }
    } catch (_) {}

    // Fallback: direct OS command
    try {
      if (Platform.isWindows) {
        if (select) {
          Process.run('explorer.exe', ['/select,$targetPath']);
        } else {
          Process.run('cmd', ['/C', 'start', '', targetPath]);
        }
      } else {
        final pth = select
            ? (File(targetPath).existsSync() ? File(targetPath).parent.path : targetPath)
            : targetPath;
        Process.run('xdg-open', [pth]);
      }
      return true;
    } catch (_) {
      return false;
    }
  }

  static File get _configFile {
    final home = Platform.environment['HOME'] ??
        Platform.environment['USERPROFILE'] ??
        '/tmp';
    return File(p.join(home, '.config', 'max-drop', 'config.json'));
  }

  Future<AppConfigModel?> fetchConfig() async {
    try {
      final resp = await http
          .get(Uri.parse('$_baseUrl/api/max/v1/config'))
          .timeout(const Duration(milliseconds: 900));
      if (resp.statusCode == 200) {
        final Map<String, dynamic> json = jsonDecode(resp.body);
        currentConfig = AppConfigModel.fromJson(json);
        notifyListeners();
        return currentConfig;
      }
    } catch (_) {}

    // Fallback to local config file
    try {
      final file = _configFile;
      if (file.existsSync()) {
        final content = await file.readAsString();
        final Map<String, dynamic> json = jsonDecode(content);
        currentConfig = AppConfigModel.fromJson(json);
        notifyListeners();
        return currentConfig;
      }
    } catch (_) {}
    return null;
  }

  Future<bool> saveConfig(AppConfigModel config) async {
    currentConfig = config;
    notifyListeners();

    bool httpOk = false;
    try {
      final resp = await http
          .post(
            Uri.parse('$_baseUrl/api/max/v1/config'),
            headers: {'Content-Type': 'application/json'},
            body: jsonEncode(config.toJson()),
          )
          .timeout(const Duration(milliseconds: 1500));
      if (resp.statusCode == 200) {
        httpOk = true;
      }
    } catch (_) {}

    // Also write to local config file directly as guarantee
    try {
      final file = _configFile;
      if (!file.parent.existsSync()) {
        file.parent.createSync(recursive: true);
      }
      const encoder = JsonEncoder.withIndent('  ');
      await file.writeAsString(encoder.convert(config.toJson()));
      return true;
    } catch (_) {
      return httpOk;
    }
  }

  Future<bool> removeTrustedDevice(String identifier) async {
    if (currentConfig != null) {
      currentConfig!.trustedDevices.remove(identifier);
      notifyListeners();
    }
    try {
      await http.post(
        Uri.parse('$_baseUrl/api/max/v1/whitelist/remove'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({'identifier': identifier}),
      );
    } catch (_) {}

    if (currentConfig != null) {
      return saveConfig(currentConfig!);
    }
    return true;
  }

  Future<void> fetchPeers() async {
    try {
      final resp = await http
          .get(Uri.parse('$_baseUrl/api/max/v1/peers'))
          .timeout(const Duration(milliseconds: 900));
      if (resp.statusCode == 200) {
        final List<dynamic> list = jsonDecode(resp.body);
        peers = list
            .map((e) => PeerDevice.fromJson(e as Map<String, dynamic>))
            .toList();
        notifyListeners();
      }
    } catch (_) {}
  }

  Future<bool> sendFiles({
    required PeerDevice target,
    required List<String> filePaths,
  }) async {
    try {
      final resp = await http.post(
        Uri.parse('$_baseUrl/api/max/v1/send'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({
          'targetIp': target.ip,
          'targetPort': target.port,
          'protocol': target.protocol,
          'files': filePaths,
        }),
      );
      return resp.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  Future<bool> respondToIncoming({
    required String sessionId,
    required bool accept,
    required bool trustAlways,
  }) async {
    try {
      final resp = await http.post(
        Uri.parse('$_baseUrl/api/max/v1/respond'),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode({
          'sessionId': sessionId,
          'accept': accept,
          'trustAlways': trustAlways,
        }),
      );
      if (!accept) {
        activeIncomingRequest = null;
        notifyListeners();
      }
      return resp.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  Future<void> openDownloadsFolder() async {
    try {
      final home = Platform.environment['HOME'] ??
          Platform.environment['USERPROFILE'] ??
          '/tmp';
      final descargas = Directory(p.join(home, 'Descargas'));
      final downloads = Directory(p.join(home, 'Downloads'));
      final targetPath = descargas.existsSync()
          ? descargas.path
          : (downloads.existsSync() ? downloads.path : home);

      if (Platform.isWindows) {
        await Process.run('explorer.exe', [targetPath]);
      } else {
        await Process.run('xdg-open', [targetPath]);
      }
    } catch (_) {}
  }

  @override
  void dispose() {
    _wsChannel?.sink.close();
    _reconnectTimer?.cancel();
    _pollTimer?.cancel();
    super.dispose();
  }
}
