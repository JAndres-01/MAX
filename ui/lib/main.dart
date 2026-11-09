import 'dart:async';
import 'dart:io';
import 'dart:ui';
import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:path/path.dart' as p;
import 'engine/max_engine_bridge.dart';
import 'engine/sound_engine.dart';
import 'settings_main.dart';

void main(List<String> args) {
  WidgetsFlutterBinding.ensureInitialized();
  if (args.contains('--settings')) {
    runApp(const MaxSettingsApp());
  } else {
    runApp(const MaxNotchApp());
  }
}

enum NotchMode {
  idle,
  staging,
  receivePrompt,
  transferring,
  completed,
}

enum MaxIconType {
  phone,
  laptop,
  tablet,
  radar,
  file,
  filePdf,
  fileImage,
  fileText,
  fileMedia,
  fileCode,
  fileArchive,
  plus,
  close,
  arrowUpRight,
  arrowDownLeft,
  check,
  error,
}

class _FileTypeVisual {
  final String extensionTag;
  final String categoryLabel;
  final Color accentColor;
  final MaxIconType iconType;

  const _FileTypeVisual({
    required this.extensionTag,
    required this.categoryLabel,
    required this.accentColor,
    required this.iconType,
  });

  static _FileTypeVisual fromFileName(String fileName) {
    final rawExt = p.extension(fileName).replaceAll('.', '').trim().toLowerCase();
    final tag = rawExt.isEmpty
        ? 'FILE'
        : (rawExt.length > 6 ? rawExt.substring(0, 6).toUpperCase() : rawExt.toUpperCase());

    switch (rawExt) {
      case 'pdf':
      case 'epub':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'DOCUMENTO',
          accentColor: const Color(0xFFD98282), // Muted dusty rose/terracotta
          iconType: MaxIconType.filePdf,
        );

      case 'txt':
      case 'md':
      case 'rtf':
      case 'doc':
      case 'docx':
      case 'odt':
      case 'log':
      case 'pages':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'TEXTO',
          accentColor: const Color(0xFFD4C5A9), // Muted warm parchment
          iconType: MaxIconType.fileText,
        );

      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'webp':
      case 'gif':
      case 'svg':
      case 'bmp':
      case 'heic':
      case 'avif':
      case 'ico':
      case 'raw':
      case 'tiff':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'IMAGEN',
          accentColor: const Color(0xFF84C2A5), // Muted sage/jade
          iconType: MaxIconType.fileImage,
        );

      case 'mp4':
      case 'mkv':
      case 'mov':
      case 'webm':
      case 'avi':
      case 'm4v':
      case 'flv':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'VIDEO',
          accentColor: const Color(0xFFAF98D6), // Muted lavender
          iconType: MaxIconType.fileMedia,
        );

      case 'mp3':
      case 'wav':
      case 'flac':
      case 'ogg':
      case 'm4a':
      case 'aac':
      case 'opus':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'AUDIO',
          accentColor: const Color(0xFFD6B378), // Muted warm amber
          iconType: MaxIconType.fileMedia,
        );

      case 'dart':
      case 'rs':
      case 'py':
      case 'js':
      case 'ts':
      case 'tsx':
      case 'jsx':
      case 'json':
      case 'yaml':
      case 'yml':
      case 'toml':
      case 'xml':
      case 'html':
      case 'css':
      case 'c':
      case 'cpp':
      case 'h':
      case 'go':
      case 'java':
      case 'kt':
      case 'sh':
      case 'sql':
      case 'ini':
      case 'conf':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'CODIGO',
          accentColor: const Color(0xFF85B6D4), // Muted frost blue
          iconType: MaxIconType.fileCode,
        );

      case 'csv':
      case 'xls':
      case 'xlsx':
      case 'ods':
      case 'tsv':
      case 'ppt':
      case 'pptx':
      case 'odp':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'DATOS',
          accentColor: const Color(0xFF98C28A), // Muted moss green
          iconType: MaxIconType.fileText,
        );

      case 'zip':
      case 'tar':
      case 'gz':
      case 'xz':
      case '7z':
      case 'rar':
      case 'bz2':
      case 'zst':
      case 'iso':
      case 'dmg':
      case 'deb':
      case 'rpm':
      case 'apk':
      case 'appimage':
      case 'exe':
      case 'msi':
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'PAQUETE',
          accentColor: const Color(0xFFCC997E), // Muted bronze/copper
          iconType: MaxIconType.fileArchive,
        );

      default:
        return _FileTypeVisual(
          extensionTag: tag,
          categoryLabel: 'ARCHIVO',
          accentColor: const Color(0xFFB8B8C2), // Muted silver
          iconType: MaxIconType.file,
        );
    }
  }
}

class StagedFileItem {
  final String id;
  final String path;
  final String name;
  final int sizeBytes;

  StagedFileItem({
    required this.id,
    required this.path,
    required this.name,
    required this.sizeBytes,
  });
}

/// Custom scroll physics that keeps a subtle spring bounce on release
/// while strictly bounding lateral overscroll (~18px max) so dragging past
/// the first or last file never exposes a large empty gap.
class _BoundedSpringScrollPhysics extends BouncingScrollPhysics {
  final double maxOverscroll;

  const _BoundedSpringScrollPhysics({
    this.maxOverscroll = 18.0,
    super.parent,
  });

  @override
  _BoundedSpringScrollPhysics applyTo(ScrollPhysics? ancestor) {
    return _BoundedSpringScrollPhysics(
      maxOverscroll: maxOverscroll,
      parent: buildParent(ancestor),
    );
  }

  @override
  double applyPhysicsToUserOffset(ScrollMetrics position, double offset) {
    if (!position.hasPixels) return 0.0;

    final double currentPixels = position.pixels;
    final double minExtent = position.minScrollExtent;
    final double maxExtent = position.maxScrollExtent;
    final double proposedPixels = currentPixels - offset;

    if (currentPixels < minExtent || proposedPixels < minExtent) {
      final double over = (minExtent - currentPixels).clamp(0.0, maxOverscroll);
      final double ratio = (over / maxOverscroll).clamp(0.0, 1.0);
      return offset * 0.24 * (1.0 - ratio * 0.88);
    }

    if (currentPixels > maxExtent || proposedPixels > maxExtent) {
      final double over = (currentPixels - maxExtent).clamp(0.0, maxOverscroll);
      final double ratio = (over / maxOverscroll).clamp(0.0, 1.0);
      return offset * 0.24 * (1.0 - ratio * 0.88);
    }

    return offset;
  }

  @override
  double applyBoundaryConditions(ScrollMetrics position, double value) {
    final double minAllowed = position.minScrollExtent - maxOverscroll;
    final double maxAllowed = position.maxScrollExtent + maxOverscroll;

    if (value < minAllowed) {
      if (position.pixels <= minAllowed) {
        return value - position.pixels;
      }
      return value - minAllowed;
    }

    if (value > maxAllowed) {
      if (position.pixels >= maxAllowed) {
        return value - position.pixels;
      }
      return value - maxAllowed;
    }

    return 0.0;
  }
}

class MaxNotchApp extends StatelessWidget {
  const MaxNotchApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'MAX',
      debugShowCheckedModeBanner: false,
      scrollBehavior: const MaterialScrollBehavior().copyWith(
        dragDevices: {
          PointerDeviceKind.mouse,
          PointerDeviceKind.touch,
          PointerDeviceKind.trackpad,
          PointerDeviceKind.stylus,
        },
        scrollbars: false,
      ),
      theme: ThemeData.dark().copyWith(
        scaffoldBackgroundColor: Colors.transparent,
      ),
      home: const Scaffold(
        backgroundColor: Colors.transparent,
        body: MaxNotchShell(),
      ),
    );
  }
}

class MaxNotchShell extends StatefulWidget {
  const MaxNotchShell({super.key});

  @override
  State<MaxNotchShell> createState() => _MaxNotchShellState();
}

class _MaxNotchShellState extends State<MaxNotchShell>
    with TickerProviderStateMixin {
  static const MethodChannel _windowChannel = MethodChannel('max/window');
  final MaxEngineBridge _bridge = MaxEngineBridge();

  NotchMode _mode = NotchMode.idle;
  NotchMode _activeContentMode = NotchMode.staging;

  final List<StagedFileItem> _stagedFiles = [];
  StagedFileItem? _hoveredStagedFile;
  IncomingFileDto? _hoveredIncomingFile;
  int _selectedPeerIndex = 0;
  int? _hoveredPeerIndex;

  final ScrollController _queueScrollController = ScrollController();
  bool _showLeftQueueFade = false;
  bool _showRightQueueFade = false;

  bool _isCursorInside = false;
  bool _isDraggingOver = false;
  bool _trustAlwaysToggle = false;
  bool _isReceivingFlow = false;
  bool _lastTransferSucceeded = true;
  String _transferPartnerName = '';
  double _transferProgress = 0.0;
  double _transferSpeedMbps = 0.0;
  String _completionTitle = '';
  String _completionSubtitle = '';
  String? _lastReceivedFilePath;
  String? _lastReceivedFolderPath;
  int _lastReceivedFileCount = 1;
  int _lastTotalFileCount = 1;
  bool _isPartialTransfer = false;
  int _transferFilesCompleted = 0;
  int _transferTotalFiles = 1;

  /// Controls the 1.0-second hover charge on the top strip
  late final AnimationController _hoverHoldController;

  /// Controls the physical unfolding/retraction of the panel from the top strip
  late final AnimationController _morphController;
  late final Animation<double> _morphHeightFactor;
  late final Animation<double> _morphWidthFactor;
  late final Animation<double> _morphContentOpacity;

  /// Controls the subtle pulse on the Nothing OS matrix progress bar
  late final AnimationController _matrixPulseController;

  Timer? _autoHideTimer;
  Timer? _completionHoldTimer;

  void _updateQueueEdgeFades() {
    if (!mounted) return;
    if (!_queueScrollController.hasClients || _stagedFiles.isEmpty) {
      if (_showLeftQueueFade || _showRightQueueFade) {
        setState(() {
          _showLeftQueueFade = false;
          _showRightQueueFade = false;
        });
      }
      return;
    }

    final pos = _queueScrollController.position;
    if (!pos.hasContentDimensions) return;

    final bool canLeft = pos.pixels > 3.0;
    final bool canRight = (pos.maxScrollExtent - pos.pixels) > 3.0;

    if (canLeft != _showLeftQueueFade || canRight != _showRightQueueFade) {
      setState(() {
        _showLeftQueueFade = canLeft;
        _showRightQueueFade = canRight;
      });
    }
  }

  void _scheduleQueueFadeCheck() {
    WidgetsBinding.instance.addPostFrameCallback((_) => _updateQueueEdgeFades());
  }

  void _syncInputRegion() {
    int width = 240;
    int height = 24;

    if (_mode != NotchMode.idle || _morphController.value > 0.01) {
      final effective = _mode == NotchMode.idle ? _activeContentMode : _mode;
      switch (effective) {
        case NotchMode.staging:
          width = 468;
          height = _stagedFiles.isEmpty ? 150 : 220;
          break;
        case NotchMode.receivePrompt:
          width = 468;
          height = 206;
          break;
        case NotchMode.transferring:
          width = _isReceivingFlow ? 468 : 398;
          height = _transferTotalFiles > 1 ? 122 : 102;
          break;
        case NotchMode.completed:
          width = _isReceivingFlow ? 468 : 398;
          height = 102;
          break;
        case NotchMode.idle:
          width = 240;
          height = 24;
          break;
      }
    }

    _windowChannel.invokeMethod('setInputRegion', {
      'width': width,
      'height': height,
    }).catchError((_) {});
  }

  @override
  void initState() {
    super.initState();

    _queueScrollController.addListener(_updateQueueEdgeFades);

    _hoverHoldController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1000),
    )..addStatusListener((status) {
        if (status == AnimationStatus.completed && _mode == NotchMode.idle) {
          _openPanelWithMode(NotchMode.staging);
        }
      });

    _morphController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 210),
      reverseDuration: const Duration(milliseconds: 170),
    )
      ..addListener(() {
        setState(() {});
      })
      ..addStatusListener((status) {
        if (status == AnimationStatus.dismissed && _mode == NotchMode.idle) {
          _syncInputRegion();
          setState(() {});
        }
      });

    _morphHeightFactor = CurvedAnimation(
      parent: _morphController,
      curve: Curves.easeOutQuart,
      reverseCurve: Curves.easeInCubic,
    );

    _morphWidthFactor = CurvedAnimation(
      parent: _morphController,
      curve: const Interval(0.0, 0.85, curve: Curves.easeOutQuart),
      reverseCurve: const Interval(0.1, 1.0, curve: Curves.easeInCubic),
    );

    _morphContentOpacity = CurvedAnimation(
      parent: _morphController,
      curve: const Interval(0.05, 0.75, curve: Curves.easeOut),
      reverseCurve: const Interval(0.25, 1.0, curve: Curves.easeIn),
    );

    _matrixPulseController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 700),
    )..repeat(reverse: true);

    _bridge.addListener(_onBridgeUpdate);
    _bridge.initialize();
    WidgetsBinding.instance.addPostFrameCallback((_) => _syncInputRegion());
  }

  @override
  void dispose() {
    _queueScrollController.removeListener(_updateQueueEdgeFades);
    _queueScrollController.dispose();
    _hoverHoldController.dispose();
    _morphController.dispose();
    _matrixPulseController.dispose();
    _autoHideTimer?.cancel();
    _completionHoldTimer?.cancel();
    _bridge.removeListener(_onBridgeUpdate);
    _bridge.dispose();
    super.dispose();
  }

  bool get _isVisibilityLocked =>
      _mode == NotchMode.transferring ||
      _mode == NotchMode.completed ||
      _mode == NotchMode.receivePrompt;

  void _openPanelWithMode(NotchMode targetMode) {
    _autoHideTimer?.cancel();
    _hoverHoldController.reset();
    setState(() {
      _mode = targetMode;
      _activeContentMode = targetMode;
    });
    _syncInputRegion();
    _morphController.forward();
  }

  void _closePanelToIdle() {
    if (_mode == NotchMode.idle) return;
    setState(() {
      _mode = NotchMode.idle;
    });
    _morphController.reverse();
  }

  void _onBridgeUpdate() {
    if (!mounted) return;

    // 1. Incoming file transfer request (confirmation prompt)
    if (_bridge.activeIncomingRequest != null &&
        _mode != NotchMode.transferring &&
        _mode != NotchMode.receivePrompt) {
      _autoHideTimer?.cancel();
      _completionHoldTimer?.cancel();
      _trustAlwaysToggle = false;
      _isReceivingFlow = true;
      _transferPartnerName = _bridge.activeIncomingRequest!.senderAlias;
      _lastReceivedFileCount = _bridge.activeIncomingRequest!.files.isNotEmpty
          ? _bridge.activeIncomingRequest!.files.length
          : 1;
      _lastTotalFileCount = _lastReceivedFileCount;
      _transferTotalFiles = _lastReceivedFileCount;
      _transferFilesCompleted = 0;
      _isPartialTransfer = false;
      _openPanelWithMode(NotchMode.receivePrompt);
      return;
    }

    // 2. Real-time transfer progress from Rust Core
    final prog = _bridge.currentProgress;
    if (prog != null) {
      if (prog.sessionId.startsWith('recv::')) {
        final parts = prog.sessionId.split('::');
        _isReceivingFlow = true;
        if (parts.length > 1 && parts[1].isNotEmpty) {
          _transferPartnerName = parts[1];
        }
      }
      if (_mode != NotchMode.transferring) {
        _autoHideTimer?.cancel();
        _completionHoldTimer?.cancel();
        _openPanelWithMode(NotchMode.transferring);
      }
      setState(() {
        _transferProgress = prog.fraction;
        _transferSpeedMbps = prog.speedMbps;
        _transferFilesCompleted = prog.filesCompleted;
        _transferTotalFiles = prog.totalFiles;
      });
    }

    // 3. Transfer completion / cancellation / error from Rust Core
    final comp = _bridge.lastCompletion;
    if (comp != null) {
      _bridge.lastCompletion = null;
      _bridge.currentProgress = null;
      final bool isRecvEvent = comp.sessionId.startsWith('recv::') || comp.isIncoming;
      if (isRecvEvent) {
        final parts = comp.sessionId.split('::');
        _isReceivingFlow = true;
        if (parts.length > 1 && parts[1].isNotEmpty) {
          _transferPartnerName = parts[1];
        }
        if (comp.filePath != null) {
          _lastReceivedFilePath = comp.filePath;
        }
        if (comp.folderPath != null) {
          _lastReceivedFolderPath = comp.folderPath;
        }
        if (comp.fileCount > 0) {
          _lastReceivedFileCount = comp.fileCount;
        }
        if (comp.totalFileCount > 0) {
          _lastTotalFileCount = comp.totalFileCount;
        }
      }
      if (isRecvEvent ||
          _mode == NotchMode.transferring ||
          _mode == NotchMode.receivePrompt) {
        _finishTransferAnimation(
          success: comp.success,
          customMessage: comp.message,
          filePath: comp.filePath,
          folderPath: comp.folderPath,
          fileCount: comp.fileCount,
          totalFileCount: comp.totalFileCount,
          isPartial: comp.isPartial,
        );
      }
    }

    // 4. Dynamic settings synchronization from Rust Core
    final cfg = _bridge.currentConfig;
    if (cfg != null) {
      final newDuration = Duration(milliseconds: cfg.hoverDelayMs);
      if (_hoverHoldController.duration != newDuration) {
        _hoverHoldController.duration = newDuration;
      }
    }

    setState(() {});
  }

  void _scheduleHideIfUnlocked() {
    _autoHideTimer?.cancel();
    if (_isVisibilityLocked || _isDraggingOver || _isCursorInside) return;

    final hideDelay = _bridge.currentConfig?.autoHideDelayMs ?? 420;
    _autoHideTimer = Timer(Duration(milliseconds: hideDelay), () {
      if (!mounted) return;
      if (!_isVisibilityLocked && !_isDraggingOver && !_isCursorInside) {
        _closePanelToIdle();
      }
    });
  }

  Future<void> _addDroppedFiles(List<String> paths) async {
    bool addedAny = false;
    for (final filePath in paths) {
      try {
        final file = File(filePath);
        if (await file.exists()) {
          final stat = await file.stat();
          final item = StagedFileItem(
            id: '${DateTime.now().microsecondsSinceEpoch}_${p.basename(filePath)}',
            path: filePath,
            name: p.basename(filePath),
            sizeBytes: stat.size,
          );
          setState(() {
            _stagedFiles.insert(0, item);
          });
          addedAny = true;
          _syncInputRegion();
          _scheduleQueueFadeCheck();
        }
      } catch (_) {}
    }
    if (addedAny) {
      SoundEngine.playSnap(bridge: _bridge);
    }
  }

  Future<void> _triggerSend() async {
    // Strictly block sending if there are no files OR no available devices
    if (_stagedFiles.isEmpty || _bridge.peers.isEmpty) return;

    final peers = _bridge.peers;
    final target = peers[_selectedPeerIndex.clamp(0, peers.length - 1)];

    _autoHideTimer?.cancel();
    _completionHoldTimer?.cancel();

    final paths = _stagedFiles.map((e) => e.path).toList();

    setState(() {
      // Immediately clear staged files upon pressing SEND
      _stagedFiles.clear();
      _hoveredStagedFile = null;
      _showLeftQueueFade = false;
      _showRightQueueFade = false;
      _isReceivingFlow = false;
      _lastReceivedFileCount = 1;
      _lastTotalFileCount = paths.length;
      _isPartialTransfer = false;
      _transferFilesCompleted = 0;
      _transferTotalFiles = paths.length;
      _lastReceivedFolderPath = null;
      _transferPartnerName = target.alias;
      _transferProgress = 0.0;
      _transferSpeedMbps = 0.0;
      _mode = NotchMode.transferring;
      _activeContentMode = NotchMode.transferring;
    });
    _syncInputRegion();

    final ok = await _bridge.sendFiles(target: target, filePaths: paths);
    if (!ok && mounted && _mode == NotchMode.transferring) {
      _finishTransferAnimation(success: false);
    }
  }

  Future<void> _respondToIncoming(bool accept) async {
    final req = _bridge.activeIncomingRequest;
    if (req == null) return;

    await _bridge.respondToIncoming(
      sessionId: req.sessionId,
      accept: accept,
      trustAlways: _trustAlwaysToggle,
    );

    if (!accept) {
      _bridge.activeIncomingRequest = null;
      SoundEngine.playClick(bridge: _bridge);
      _closePanelToIdle();
      return;
    }

    setState(() {
      _isReceivingFlow = true;
      _transferPartnerName = req.senderAlias;
      _transferProgress = 0.0;
      _transferSpeedMbps = 0.0;
      final fileCount = req.files.isNotEmpty ? req.files.length : 1;
      _lastReceivedFileCount = fileCount;
      _lastTotalFileCount = fileCount;
      _transferTotalFiles = fileCount;
      _transferFilesCompleted = 0;
      _isPartialTransfer = false;
      _bridge.activeIncomingRequest = null;
      _mode = NotchMode.transferring;
      _activeContentMode = NotchMode.transferring;
    });
    _syncInputRegion();
  }

  void _finishTransferAnimation({
    required bool success,
    String? customMessage,
    String? filePath,
    String? folderPath,
    int? fileCount,
    int? totalFileCount,
    bool isPartial = false,
  }) {
    if (!mounted) return;
    _autoHideTimer?.cancel();
    _hoverHoldController.reset();

    if (filePath != null && filePath.isNotEmpty) {
      _lastReceivedFilePath = filePath;
    }
    if (folderPath != null && folderPath.isNotEmpty) {
      _lastReceivedFolderPath = folderPath;
    }
    if (fileCount != null && fileCount > 0) {
      _lastReceivedFileCount = fileCount;
    }
    if (totalFileCount != null && totalFileCount > 0) {
      _lastTotalFileCount = totalFileCount;
    }
    final int safeTotal =
        _lastTotalFileCount > 0 ? _lastTotalFileCount : _transferTotalFiles;
    _isPartialTransfer =
        isPartial || (!success && _lastReceivedFileCount > 0 && safeTotal > 1);

    if (success || _isPartialTransfer) {
      SoundEngine.playComplete(bridge: _bridge);
    } else {
      SoundEngine.playClick(bridge: _bridge);
    }

    setState(() {
      _lastTransferSucceeded = success;
      if (success) {
        if (_isReceivingFlow) {
          _completionTitle = (_lastReceivedFileCount > 1)
              ? 'ARCHIVOS RECIBIDOS'
              : 'ARCHIVO RECIBIDO';
          _completionSubtitle =
              (customMessage != null && customMessage.isNotEmpty)
                  ? customMessage
                  : (_lastReceivedFileCount > 1)
                      ? '${_transferPartnerName.toUpperCase()} TE ENVIO $_lastReceivedFileCount ARCHIVOS'
                      : '${_transferPartnerName.toUpperCase()} TE ENVIO UN ARCHIVO';
        } else {
          _completionTitle = 'TRANSFERENCIA COMPLETADA';
          _completionSubtitle =
              'ENVIADO A ${_transferPartnerName.toUpperCase()}';
        }
      } else if (_isPartialTransfer) {
        _completionTitle = 'TRANSFERENCIA PARCIAL';
        final failedCount =
            (safeTotal - _lastReceivedFileCount).clamp(1, safeTotal);
        _completionSubtitle = (customMessage != null && customMessage.isNotEmpty)
            ? customMessage
            : '$_lastReceivedFileCount DE $safeTotal RECIBIDOS • $failedCount CANCELADO${failedCount > 1 ? 'S' : ''}';
      } else {
        _completionTitle = 'LA TRANSFERENCIA FALLO';
        _completionSubtitle = (customMessage != null && customMessage.isNotEmpty)
            ? customMessage
            : 'CANCELADA O INTERRUMPIDA';
      }
      _mode = NotchMode.completed;
      _activeContentMode = NotchMode.completed;
    });
    _syncInputRegion();
    _morphController.forward();

    _completionHoldTimer?.cancel();
    _completionHoldTimer = Timer(
      Duration(milliseconds: (_isReceivingFlow || _isPartialTransfer) ? 4800 : 2200),
      () {
        if (!mounted) return;
        if (_isCursorInside && _stagedFiles.isNotEmpty) {
          setState(() {
            _mode = NotchMode.staging;
            _activeContentMode = NotchMode.staging;
          });
          _syncInputRegion();
        } else {
          _closePanelToIdle();
        }
      },
    );
  }

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1048576) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1073741824) {
      return '${(bytes / 1048576).toStringAsFixed(1)} MB';
    }
    return '${(bytes / 1073741824).toStringAsFixed(2)} GB';
  }

  MaxIconType _deviceIconType(String type) {
    switch (type.toLowerCase()) {
      case 'desktop':
      case 'laptop':
      case 'computer':
        return MaxIconType.laptop;
      case 'tablet':
        return MaxIconType.tablet;
      default:
        return MaxIconType.phone;
    }
  }

  double _targetExpandedWidth(NotchMode mode) {
    if (mode == NotchMode.transferring || mode == NotchMode.completed) {
      return 384.0;
    }
    return 448.0;
  }

  @override
  Widget build(BuildContext context) {
    return DropTarget(
      onEnter: (_) {
        _isDraggingOver = true;
        _autoHideTimer?.cancel();
        _hoverHoldController.reset();
        if (_mode == NotchMode.idle) {
          _openPanelWithMode(NotchMode.staging);
        } else {
          setState(() {});
        }
      },
      onLeave: () {
        _isDraggingOver = false;
        _scheduleHideIfUnlocked();
        setState(() {});
      },
      onDrop: (details) async {
        _isDraggingOver = false;
        final paths = <String>[];
        for (final item in details.items) {
          if (item is DropFileItem && item.localPath != null) {
            paths.add(item.localPath!);
          } else if (item is DropDirectoryItem && item.localPath != null) {
            paths.add(item.localPath!);
          }
        }
        if (_mode == NotchMode.idle) {
          _openPanelWithMode(NotchMode.staging);
        }
        await _addDroppedFiles(paths);
      },
      child: Align(
        alignment: Alignment.topCenter,
        child: MouseRegion(
          onEnter: (_) {
            _isCursorInside = true;
            _autoHideTimer?.cancel();
            if (_mode == NotchMode.idle && !_morphController.isAnimating) {
              _hoverHoldController.forward(from: 0.0);
            }
          },
          onExit: (_) {
            _isCursorInside = false;
            if (_mode == NotchMode.idle) {
              _hoverHoldController.reset();
            } else {
              _scheduleHideIfUnlocked();
            }
          },
          child: _buildUnifiedMorphingNotch(),
        ),
      ),
    );
  }

  /// Single unified container that physically unfolds from and retracts into the top strip.
  Widget _buildUnifiedMorphingNotch() {
    final double morphT = _morphController.value;

    if (morphT <= 0.001 && _mode == NotchMode.idle) {
      return Container(
        width: 240,
        height: 24,
        alignment: Alignment.topCenter,
        color: Colors.transparent,
        child: _buildTopStripSpine(isExpanded: false),
      );
    }

    final NotchMode displayMode =
        _mode == NotchMode.idle ? _activeContentMode : _mode;
    final double expandedWidth = _targetExpandedWidth(displayMode);
    final double stripBaseWidth =
        _bridge.currentConfig?.notchIdleWidth ?? 144.0;
    final double currentWidth = lerpDouble(
      stripBaseWidth,
      expandedWidth,
      _morphWidthFactor.value,
    )!;
    final double bottomRadius =
        lerpDouble(4.0, 16.0, _morphHeightFactor.value)!;

    return ClipRRect(
      borderRadius: BorderRadius.vertical(
        bottom: Radius.circular(bottomRadius),
      ),
      child: BackdropFilter(
        filter: ImageFilter.blur(sigmaX: 20, sigmaY: 20),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 180),
          curve: Curves.easeOutQuart,
          width: currentWidth,
          decoration: BoxDecoration(
            color: const Color(0xF509090B),
            borderRadius: BorderRadius.vertical(
              bottom: Radius.circular(bottomRadius),
            ),
            border: Border.all(
              color: _isDraggingOver
                  ? Colors.white.withValues(alpha: 0.9)
                  : Colors.white.withValues(
                      alpha: lerpDouble(0.0, 0.16, _morphHeightFactor.value)!,
                    ),
              width: 1.0,
            ),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withValues(
                  alpha: 0.72 * _morphHeightFactor.value,
                ),
                blurRadius: 24 * _morphHeightFactor.value,
                offset: Offset(0, 10 * _morphHeightFactor.value),
              ),
            ],
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              _buildTopStripSpine(isExpanded: true),
              ClipRect(
                child: Align(
                  alignment: Alignment.topCenter,
                  heightFactor: _morphHeightFactor.value.clamp(0.0, 1.0),
                  child: Opacity(
                    opacity: _morphContentOpacity.value.clamp(0.0, 1.0),
                    child: Transform.translate(
                      offset: Offset(
                        0,
                        -8.0 * (1.0 - _morphHeightFactor.value),
                      ),
                      child: AnimatedSize(
                        duration: const Duration(milliseconds: 180),
                        curve: Curves.easeOutQuart,
                        alignment: Alignment.topCenter,
                        child: Padding(
                          padding: EdgeInsets.fromLTRB(
                            18,
                            displayMode == NotchMode.transferring ||
                                    displayMode == NotchMode.completed
                                ? 12
                                : 13,
                            18,
                            displayMode == NotchMode.transferring ||
                                    displayMode == NotchMode.completed
                                ? 14
                                : 16,
                          ),
                          child: AnimatedSwitcher(
                            duration: const Duration(milliseconds: 170),
                            switchInCurve: Curves.easeOutQuart,
                            switchOutCurve: Curves.easeInCubic,
                            transitionBuilder: (child, animation) {
                              final slide = Tween<Offset>(
                                begin: const Offset(0.0, -0.10),
                                end: Offset.zero,
                              ).animate(animation);
                              return FadeTransition(
                                opacity: animation,
                                child: SlideTransition(
                                  position: slide,
                                  child: SizeTransition(
                                    sizeFactor: animation,
                                    axis: Axis.vertical,
                                    alignment: Alignment.topCenter,
                                    child: child,
                                  ),
                                ),
                              );
                            },
                            child: _buildModeContent(displayMode),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  /// Monochrome Nothing OS White/Silver top strip (zero blue).
  Widget _buildTopStripSpine({required bool isExpanded}) {
    return AnimatedBuilder(
      animation: _hoverHoldController,
      builder: (context, _) {
        final isCharging = _hoverHoldController.value > 0.0;
        final double baseW = _bridge.currentConfig?.notchIdleWidth ?? 144.0;
        final double width =
            isExpanded ? double.infinity : (isCharging ? baseW + 36.0 : baseW);
        final double height = isExpanded ? 1.5 : (isCharging ? 4.0 : 3.0);

        return AnimatedContainer(
          duration: const Duration(milliseconds: 140),
          curve: Curves.easeOutQuart,
          width: width,
          height: height,
          decoration: BoxDecoration(
            color: isExpanded
                ? Colors.white.withValues(alpha: 0.14)
                : Colors.white.withValues(alpha: isCharging ? 0.35 : 0.52),
            borderRadius: isExpanded
                ? BorderRadius.zero
                : const BorderRadius.vertical(bottom: Radius.circular(4)),
            boxShadow: isExpanded
                ? null
                : [
                    BoxShadow(
                      color: Colors.white.withValues(
                        alpha: isCharging ? 0.65 : 0.25,
                      ),
                      blurRadius: isCharging ? 10 : 5,
                    ),
                  ],
          ),
          alignment: Alignment.center,
          child: !isExpanded && isCharging
              ? FractionallySizedBox(
                  widthFactor: _hoverHoldController.value,
                  child: Container(
                    decoration: const BoxDecoration(
                      color: Colors.white,
                      borderRadius: BorderRadius.vertical(
                        bottom: Radius.circular(4),
                      ),
                    ),
                  ),
                )
              : null,
        );
      },
    );
  }

  Widget _buildModeContent(NotchMode mode) {
    switch (mode) {
      case NotchMode.staging:
        return KeyedSubtree(
          key: const ValueKey('mode_staging'),
          child: _buildStagingView(),
        );
      case NotchMode.receivePrompt:
        return KeyedSubtree(
          key: const ValueKey('mode_receive'),
          child: _buildReceivePromptView(),
        );
      case NotchMode.transferring:
        return KeyedSubtree(
          key: const ValueKey('mode_transferring'),
          child: _buildTransferringView(),
        );
      case NotchMode.completed:
        return KeyedSubtree(
          key: const ValueKey('mode_completed'),
          child: _buildCompletedView(),
        );
      case NotchMode.idle:
        return const SizedBox.shrink(key: ValueKey('mode_idle'));
    }
  }

  Widget _buildStagingView() {
    final peers = _bridge.peers;
    final bool hasPeers = peers.isNotEmpty;
    final int safeSelected =
        hasPeers ? _selectedPeerIndex.clamp(0, peers.length - 1) : 0;
    final int activeIndex = (hasPeers &&
            _hoveredPeerIndex != null &&
            _hoveredPeerIndex! < peers.length)
        ? _hoveredPeerIndex!
        : safeSelected;
    final PeerDevice? currentPeer = hasPeers ? peers[activeIndex] : null;
    final int totalBytes =
        _stagedFiles.fold<int>(0, (sum, item) => sum + item.sizeBytes);

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Header: Device Icon + Name + Peer Microdots (left) | File Count & Total Size (right)
        Row(
          children: [
            Expanded(
              child: Row(
                children: [
                  Container(
                    width: 28,
                    height: 28,
                    decoration: BoxDecoration(
                      color: const Color(0xFF141418),
                      borderRadius: BorderRadius.circular(7),
                      border: Border.all(
                        color: Colors.white.withValues(alpha: 0.18),
                      ),
                    ),
                    alignment: Alignment.center,
                    child: _MaxVectorIcon(
                      type: currentPeer != null
                          ? _deviceIconType(currentPeer.deviceType)
                          : MaxIconType.radar,
                      size: 15,
                      color: currentPeer != null
                          ? Colors.white
                          : Colors.white.withValues(alpha: 0.55),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Flexible(
                    child: AnimatedSwitcher(
                      duration: const Duration(milliseconds: 140),
                      child: Text(
                        currentPeer != null
                            ? currentPeer.alias
                            : 'Buscando dispositivos...',
                        key: ValueKey(currentPeer?.alias ?? 'searching'),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          color: currentPeer != null
                              ? Colors.white
                              : const Color(0xFF8E8E93),
                          fontSize: 13,
                          fontWeight: FontWeight.w600,
                          letterSpacing: 0.2,
                        ),
                      ),
                    ),
                  ),
                  if (hasPeers) ...[
                    const SizedBox(width: 10),
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: List.generate(peers.length, (idx) {
                        final isSelected = idx == activeIndex;
                        return MouseRegion(
                          onEnter: (_) =>
                              setState(() => _hoveredPeerIndex = idx),
                          onExit: (_) =>
                              setState(() => _hoveredPeerIndex = null),
                          cursor: SystemMouseCursors.click,
                          child: GestureDetector(
                            onTap: () =>
                                setState(() => _selectedPeerIndex = idx),
                            child: AnimatedContainer(
                              duration: const Duration(milliseconds: 160),
                              curve: Curves.easeOutQuart,
                              margin:
                                  const EdgeInsets.symmetric(horizontal: 2.5),
                              width: isSelected ? 16 : 5,
                              height: 5,
                              decoration: BoxDecoration(
                                color: isSelected
                                    ? Colors.white
                                    : Colors.white.withValues(alpha: 0.24),
                                borderRadius: BorderRadius.circular(99),
                              ),
                            ),
                          ),
                        );
                      }),
                    ),
                  ],
                ],
              ),
            ),
            AnimatedSwitcher(
              duration: const Duration(milliseconds: 140),
              child: _stagedFiles.isNotEmpty
                  ? Padding(
                      key: const ValueKey('header_queue_summary'),
                      padding: const EdgeInsets.only(left: 10),
                      child: Text(
                        '${_stagedFiles.length} ${_stagedFiles.length == 1 ? 'ARCHIVO' : 'ARCHIVOS'}  •  ${_formatBytes(totalBytes)}',
                        style: const TextStyle(
                          color: Color(0xFF8E8E93),
                          fontSize: 10.0,
                          fontWeight: FontWeight.w600,
                          letterSpacing: 0.8,
                          fontFeatures: [FontFeature.tabularFigures()],
                        ),
                      ),
                    )
                  : const SizedBox.shrink(key: ValueKey('header_queue_empty')),
            ),
          ],
        ),
        const SizedBox(height: 12),
        AnimatedSize(
          duration: const Duration(milliseconds: 180),
          curve: Curves.easeOutQuart,
          alignment: Alignment.topCenter,
          child: AnimatedSwitcher(
            duration: const Duration(milliseconds: 160),
            switchInCurve: Curves.easeOutQuart,
            switchOutCurve: Curves.easeInCubic,
            transitionBuilder: (child, animation) {
              return FadeTransition(
                opacity: animation,
                child: SizeTransition(
                  sizeFactor: animation,
                  axis: Axis.vertical,
                  alignment: Alignment.topCenter,
                  child: child,
                ),
              );
            },
            child: _stagedFiles.isEmpty
                ? _buildNothingEmptyDropZone()
                : _buildStagedFilesAndFooter(hasPeers),
          ),
        ),
      ],
    );
  }

  Widget _buildNothingEmptyDropZone() {
    return Container(
      key: const ValueKey('empty_drop_zone'),
      height: 62,
      decoration: BoxDecoration(
        color: const Color(0xFF111114),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: _isDraggingOver
              ? Colors.white.withValues(alpha: 0.8)
              : Colors.white.withValues(alpha: 0.12),
        ),
      ),
      child: CustomPaint(
        painter: _NothingDotMatrixPainter(),
        child: Center(
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              _MaxVectorIcon(
                type: MaxIconType.plus,
                size: 13,
                color: Colors.white.withValues(alpha: 0.75),
              ),
              const SizedBox(width: 8),
              Text(
                'ARRASTRA ARCHIVOS AQUI',
                style: TextStyle(
                  color: Colors.white.withValues(alpha: 0.68),
                  fontSize: 10.5,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 1.4,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildStagedFilesAndFooter(bool hasPeers) {
    final hovered = _hoveredStagedFile != null &&
            _stagedFiles.any((f) => f.id == _hoveredStagedFile!.id)
        ? _hoveredStagedFile
        : null;
    final hoveredVisual =
        hovered != null ? _FileTypeVisual.fromFileName(hovered.name) : null;

    return Column(
      key: const ValueKey('staged_files_queue'),
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          height: 62,
          child: Stack(
            children: [
              NotificationListener<ScrollMetricsNotification>(
                onNotification: (_) {
                  _scheduleQueueFadeCheck();
                  return false;
                },
                child: SingleChildScrollView(
                  controller: _queueScrollController,
                  scrollDirection: Axis.horizontal,
                  physics: const _BoundedSpringScrollPhysics(
                    maxOverscroll: 18.0,
                  ),
                  child: Row(
                    children: [
                      for (final file in _stagedFiles)
                        _StagedFileCard(
                          key: ValueKey(file.id),
                          item: file,
                          formattedSize: _formatBytes(file.sizeBytes),
                          onHoverChanged: (isHovered) {
                            setState(() {
                              if (isHovered) {
                                _hoveredStagedFile = file;
                              } else if (_hoveredStagedFile?.id == file.id) {
                                _hoveredStagedFile = null;
                              }
                            });
                          },
                          onRemoveStart: () {
                            if (_hoveredStagedFile?.id == file.id) {
                              setState(() {
                                _hoveredStagedFile = null;
                              });
                            }
                          },
                          onRemove: () {
                            setState(() {
                              if (_hoveredStagedFile?.id == file.id) {
                                _hoveredStagedFile = null;
                              }
                              _stagedFiles.removeWhere((f) => f.id == file.id);
                            });
                            _syncInputRegion();
                            _scheduleQueueFadeCheck();
                          },
                        ),
                    ],
                  ),
                ),
              ),
              // Dark fade gradient on the left edge when more files exist to the left
              Positioned(
                left: 0,
                top: 0,
                bottom: 0,
                width: 38,
                child: IgnorePointer(
                  child: AnimatedOpacity(
                    duration: const Duration(milliseconds: 150),
                    curve: Curves.easeOutCubic,
                    opacity: _showLeftQueueFade ? 1.0 : 0.0,
                    child: Container(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.centerLeft,
                          end: Alignment.centerRight,
                          colors: [
                            const Color(0xFF09090B),
                            const Color(0xFF09090B).withValues(alpha: 0.84),
                            Colors.transparent,
                          ],
                          stops: const [0.0, 0.36, 1.0],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              // Dark fade gradient on the right edge when more files exist to the right
              Positioned(
                right: 0,
                top: 0,
                bottom: 0,
                width: 38,
                child: IgnorePointer(
                  child: AnimatedOpacity(
                    duration: const Duration(milliseconds: 150),
                    curve: Curves.easeOutCubic,
                    opacity: _showRightQueueFade ? 1.0 : 0.0,
                    child: Container(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          begin: Alignment.centerLeft,
                          end: Alignment.centerRight,
                          colors: [
                            Colors.transparent,
                            const Color(0xFF09090B).withValues(alpha: 0.84),
                            const Color(0xFF09090B),
                          ],
                          stops: const [0.0, 0.64, 1.0],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 11),
        Row(
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Expanded(
              child: AnimatedSize(
                duration: const Duration(milliseconds: 150),
                curve: Curves.easeOutQuart,
                alignment: Alignment.centerLeft,
                child: AnimatedSwitcher(
                  duration: const Duration(milliseconds: 140),
                  switchInCurve: Curves.easeOutCubic,
                  switchOutCurve: Curves.easeInCubic,
                  layoutBuilder: (currentChild, previousChildren) {
                    return Stack(
                      alignment: Alignment.centerLeft,
                      children: <Widget>[
                        ...previousChildren,
                        ?currentChild,
                      ],
                    );
                  },
                  child: hovered != null && hoveredVisual != null
                      ? Column(
                          key: ValueKey('meta_${hovered.id}'),
                          crossAxisAlignment: CrossAxisAlignment.start,
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Text(
                              hovered.name,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: const TextStyle(
                                color: Color(0xFFF4F4F5),
                                fontSize: 10.5,
                                fontWeight: FontWeight.w600,
                                height: 1.2,
                              ),
                            ),
                            const SizedBox(height: 3),
                            Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Container(
                                  width: 6,
                                  height: 6,
                                  decoration: BoxDecoration(
                                    color: hoveredVisual.accentColor,
                                    shape: BoxShape.circle,
                                  ),
                                ),
                                const SizedBox(width: 6),
                                Flexible(
                                  child: Text(
                                    '${hoveredVisual.extensionTag}  •  ${hoveredVisual.categoryLabel}  •  ${_formatBytes(hovered.sizeBytes)}',
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                    style: TextStyle(
                                      color: hoveredVisual.accentColor
                                          .withValues(alpha: 0.92),
                                      fontSize: 9.5,
                                      fontWeight: FontWeight.w600,
                                      letterSpacing: 0.7,
                                      fontFeatures: const [
                                        FontFeature.tabularFigures(),
                                      ],
                                    ),
                                  ),
                                ),
                              ],
                            ),
                          ],
                        )
                      : const SizedBox.shrink(
                          key: ValueKey('meta_idle_empty'),
                        ),
                ),
              ),
            ),
            const SizedBox(width: 10),
            MouseRegion(
              cursor:
                  hasPeers ? SystemMouseCursors.click : SystemMouseCursors.basic,
              child: GestureDetector(
                onTap: hasPeers ? _triggerSend : null,
                child: AnimatedOpacity(
                  duration: const Duration(milliseconds: 150),
                  opacity: hasPeers ? 1.0 : 0.28,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 7,
                    ),
                    decoration: BoxDecoration(
                      color: Colors.white,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(
                          hasPeers
                              ? 'SEND [${_stagedFiles.length}]'
                              : 'SIN DISPOSITIVO',
                          style: const TextStyle(
                            color: Color(0xFF09090B),
                            fontSize: 10.5,
                            fontWeight: FontWeight.w700,
                            letterSpacing: 1.0,
                            fontFeatures: [FontFeature.tabularFigures()],
                          ),
                        ),
                        if (hasPeers) ...[
                          const SizedBox(width: 6),
                          const _MaxVectorIcon(
                            type: MaxIconType.arrowUpRight,
                            size: 11,
                            color: Color(0xFF09090B),
                          ),
                        ],
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildReceivePromptView() {
    final req = _bridge.activeIncomingRequest;
    if (req == null) return const SizedBox.shrink();

    final hoveredInc = _hoveredIncomingFile;
    final hoveredIncVisual = hoveredInc != null
        ? _FileTypeVisual.fromFileName(hoveredInc.fileName)
        : null;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Container(
              width: 28,
              height: 28,
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(7),
              ),
              alignment: Alignment.center,
              child: _MaxVectorIcon(
                type: _deviceIconType(req.senderDeviceType),
                size: 15,
                color: const Color(0xFF09090B),
              ),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    req.senderAlias,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: const TextStyle(
                      color: Colors.white,
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                    ),
                  ),
                  AnimatedSwitcher(
                    duration: const Duration(milliseconds: 130),
                    layoutBuilder: (currentChild, previousChildren) {
                      return Stack(
                        alignment: Alignment.centerLeft,
                        children: <Widget>[
                          ...previousChildren,
                          ?currentChild,
                        ],
                      );
                    },
                    child: hoveredInc != null && hoveredIncVisual != null
                        ? Text(
                            key: ValueKey('inc_${hoveredInc.id}'),
                            '${hoveredInc.fileName}  •  ${hoveredIncVisual.extensionTag} (${hoveredIncVisual.categoryLabel})  •  ${_formatBytes(hoveredInc.size)}',
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              color: hoveredIncVisual.accentColor,
                              fontSize: 10,
                              fontWeight: FontWeight.w600,
                              letterSpacing: 0.4,
                              fontFeatures: const [
                                FontFeature.tabularFigures(),
                              ],
                            ),
                          )
                        : Text(
                            key: const ValueKey('inc_summary'),
                            'QUIERE ENVIARTE ${req.files.length} ${req.files.length == 1 ? 'ARCHIVO' : 'ARCHIVOS'} • ${_formatBytes(req.totalSize)}',
                            style: const TextStyle(
                              color: Color(0xFF8E8E93),
                              fontSize: 10,
                              fontWeight: FontWeight.w600,
                              letterSpacing: 0.8,
                              fontFeatures: [FontFeature.tabularFigures()],
                            ),
                          ),
                  ),
                ],
              ),
            ),
          ],
        ),
        const SizedBox(height: 10),
        SizedBox(
          height: 34,
          child: ListView.separated(
            scrollDirection: Axis.horizontal,
            itemCount: req.files.length,
            separatorBuilder: (_, _) => const SizedBox(width: 6),
            itemBuilder: (context, idx) {
              final f = req.files[idx];
              final visual = _FileTypeVisual.fromFileName(f.fileName);
              final isHovered = _hoveredIncomingFile?.id == f.id;
              return MouseRegion(
                onEnter: (_) => setState(() => _hoveredIncomingFile = f),
                onExit: (_) {
                  if (_hoveredIncomingFile?.id == f.id) {
                    setState(() => _hoveredIncomingFile = null);
                  }
                },
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 140),
                  padding:
                      const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                  decoration: BoxDecoration(
                    color: const Color(0xFF141418),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: isHovered
                          ? visual.accentColor.withValues(alpha: 0.45)
                          : visual.accentColor.withValues(alpha: 0.20),
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      _MaxVectorIcon(
                        type: visual.iconType,
                        size: 12,
                        color: visual.accentColor,
                      ),
                      const SizedBox(width: 6),
                      ConstrainedBox(
                        constraints: const BoxConstraints(maxWidth: 135),
                        child: Text(
                          f.fileName,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 11,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ),
                      const SizedBox(width: 6),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 4,
                          vertical: 1,
                        ),
                        decoration: BoxDecoration(
                          color: visual.accentColor.withValues(alpha: 0.14),
                          borderRadius: BorderRadius.circular(3),
                        ),
                        child: Text(
                          visual.extensionTag,
                          style: TextStyle(
                            color: visual.accentColor,
                            fontSize: 8.5,
                            fontWeight: FontWeight.w700,
                            letterSpacing: 0.5,
                          ),
                        ),
                      ),
                      const SizedBox(width: 5),
                      Text(
                        _formatBytes(f.size),
                        style: const TextStyle(
                          color: Color(0xFF8E8E93),
                          fontSize: 10,
                          fontFeatures: [FontFeature.tabularFigures()],
                        ),
                      ),
                    ],
                  ),
                ),
              );
            },
          ),
        ),
        const SizedBox(height: 12),
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            GestureDetector(
              onTap: () =>
                  setState(() => _trustAlwaysToggle = !_trustAlwaysToggle),
              child: MouseRegion(
                cursor: SystemMouseCursors.click,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    AnimatedContainer(
                      duration: const Duration(milliseconds: 160),
                      width: 28,
                      height: 15,
                      padding: const EdgeInsets.all(2),
                      decoration: BoxDecoration(
                        color: _trustAlwaysToggle
                            ? Colors.white
                            : const Color(0xFF27272A),
                        borderRadius: BorderRadius.circular(99),
                      ),
                      alignment: _trustAlwaysToggle
                          ? Alignment.centerRight
                          : Alignment.centerLeft,
                      child: Container(
                        width: 11,
                        height: 11,
                        decoration: BoxDecoration(
                          color: _trustAlwaysToggle
                              ? const Color(0xFF09090B)
                              : Colors.white,
                          shape: BoxShape.circle,
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    const Text(
                      'CONFIAR SIEMPRE',
                      style: TextStyle(
                        color: Color(0xFF8E8E93),
                        fontSize: 10,
                        fontWeight: FontWeight.w600,
                        letterSpacing: 0.9,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                MouseRegion(
                  cursor: SystemMouseCursors.click,
                  child: GestureDetector(
                    onTap: () => _respondToIncoming(false),
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 13,
                        vertical: 6,
                      ),
                      decoration: BoxDecoration(
                        color: const Color(0xFF141418),
                        borderRadius: BorderRadius.circular(7),
                        border: Border.all(
                          color: Colors.white.withValues(alpha: 0.14),
                        ),
                      ),
                      child: const Text(
                        'RECHAZAR',
                        style: TextStyle(
                          color: Color(0xFFA1A1AA),
                          fontSize: 10.5,
                          fontWeight: FontWeight.w600,
                          letterSpacing: 0.9,
                        ),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                MouseRegion(
                  cursor: SystemMouseCursors.click,
                  child: GestureDetector(
                    onTap: () => _respondToIncoming(true),
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 15,
                        vertical: 6,
                      ),
                      decoration: BoxDecoration(
                        color: Colors.white,
                        borderRadius: BorderRadius.circular(7),
                      ),
                      child: const Text(
                        'ACEPTAR',
                        style: TextStyle(
                          color: Color(0xFF09090B),
                          fontSize: 10.5,
                          fontWeight: FontWeight.w700,
                          letterSpacing: 0.9,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ],
        ),
      ],
    );
  }

  /// Nothing OS Segmented Matrix Transfer View (100% driven by real bytes)
  Widget _buildTransferringView() {
    final pct = (_transferProgress * 100).round().clamp(0, 100);
    final pctFormatted = pct.toString().padLeft(3, '0');
    final label = _isReceivingFlow
        ? 'RECIBIENDO  •  ${_transferPartnerName.toUpperCase()}'
        : 'ENVIANDO  •  ${_transferPartnerName.toUpperCase()}';

    const int totalSegments = 34;
    final int activeSegments =
        (_transferProgress.clamp(0.0, 1.0) * totalSegments).ceil();

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Expanded(
              child: Row(
                children: [
                  _MaxVectorIcon(
                    type: _isReceivingFlow
                        ? MaxIconType.arrowDownLeft
                        : MaxIconType.arrowUpRight,
                    size: 13,
                    color: Colors.white,
                  ),
                  const SizedBox(width: 8),
                  Flexible(
                    child: Text(
                      label,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                        letterSpacing: 1.0,
                      ),
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: 8),
            Text(
              '${_transferSpeedMbps.toStringAsFixed(1)} MB/S   $pctFormatted%',
              style: const TextStyle(
                color: Colors.white,
                fontSize: 11,
                fontWeight: FontWeight.w700,
                letterSpacing: 0.8,
                fontFeatures: [FontFeature.tabularFigures()],
              ),
            ),
          ],
        ),
        const SizedBox(height: 11),
        AnimatedBuilder(
          animation: _matrixPulseController,
          builder: (context, _) {
            return SizedBox(
              height: 8,
              child: Row(
                children: List.generate(totalSegments, (idx) {
                  final bool isLit = idx < activeSegments;
                  final bool isWaitingStart =
                      activeSegments == 0 && idx == 0;
                  final bool isLeadingEdge =
                      idx == activeSegments - 1 || isWaitingStart;
                  final double alpha = (isLit || isWaitingStart)
                      ? (isLeadingEdge
                          ? 0.45 + (_matrixPulseController.value * 0.55)
                          : 1.0)
                      : 0.14;
                  return Expanded(
                    child: Container(
                      margin: const EdgeInsets.symmetric(horizontal: 1.1),
                      decoration: BoxDecoration(
                        color: Colors.white.withValues(alpha: alpha),
                        borderRadius: BorderRadius.circular(1.5),
                      ),
                    ),
                  );
                }),
              ),
            );
          },
        ),
        if (_transferTotalFiles > 1) ...[
          const SizedBox(height: 8),
          Align(
            alignment: Alignment.centerLeft,
            child: _buildNothingOsMicroDots(),
          ),
        ],
      ],
    );
  }

  Widget _buildNothingOsMicroDots() {
    final int total = _transferTotalFiles.clamp(1, 14);
    final int completed = _transferFilesCompleted.clamp(0, total);

    return AnimatedBuilder(
      animation: _matrixPulseController,
      builder: (context, _) {
        return Row(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: [
            Row(
              mainAxisSize: MainAxisSize.min,
              children: List.generate(total, (idx) {
                final bool isDone = idx < completed;
                final bool isCurrent = idx == completed;
                final Color dotColor = isDone
                    ? Colors.white
                    : isCurrent
                        ? Colors.white.withValues(
                            alpha: 0.40 + (_matrixPulseController.value * 0.60),
                          )
                        : Colors.white.withValues(alpha: 0.16);

                return Container(
                  width: 5,
                  height: 5,
                  margin: const EdgeInsets.only(right: 3.5),
                  decoration: BoxDecoration(
                    color: dotColor,
                    borderRadius: BorderRadius.circular(1.2),
                    border: isCurrent
                        ? Border.all(
                            color: Colors.white.withValues(alpha: 0.8),
                            width: 0.8,
                          )
                        : null,
                  ),
                );
              }),
            ),
            const SizedBox(width: 6),
            Text(
              '$completed DE $total ARCHIVOS',
              style: const TextStyle(
                color: Color(0xFF8E8E93),
                fontSize: 9.5,
                fontWeight: FontWeight.w600,
                letterSpacing: 0.6,
                fontFeatures: [FontFeature.tabularFigures()],
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildCompletedView() {
    return Row(
      children: [
        Container(
          width: 28,
          height: 28,
          decoration: BoxDecoration(
            color: (_lastTransferSucceeded || _isPartialTransfer)
                ? Colors.white
                : const Color(0xFF1F1F24),
            borderRadius: BorderRadius.circular(7),
            border: (_lastTransferSucceeded || _isPartialTransfer)
                ? null
                : Border.all(
                    color: Colors.white.withValues(alpha: 0.28),
                  ),
          ),
          alignment: Alignment.center,
          child: _MaxVectorIcon(
            type: (_lastTransferSucceeded || _isPartialTransfer)
                ? MaxIconType.check
                : MaxIconType.error,
            size: 15,
            color: (_lastTransferSucceeded || _isPartialTransfer)
                ? const Color(0xFF09090B)
                : Colors.white,
          ),
        ),
        const SizedBox(width: 11),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                _completionTitle,
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 11,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 1.1,
                ),
              ),
              const SizedBox(height: 2),
              Text(
                _completionSubtitle,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: const TextStyle(
                  color: Color(0xFF8E8E93),
                  fontSize: 10,
                  fontWeight: FontWeight.w500,
                  letterSpacing: 0.5,
                ),
              ),
            ],
          ),
        ),
        if (_isReceivingFlow && (_lastTransferSucceeded || _isPartialTransfer)) ...[
          const SizedBox(width: 8),
          if (_isPartialTransfer || _lastReceivedFileCount > 1) ...[
            MouseRegion(
              cursor: SystemMouseCursors.click,
              child: GestureDetector(
                onTap: () {
                  SoundEngine.playClick(bridge: _bridge);
                  if (_lastReceivedFolderPath != null) {
                    _bridge.openPath(_lastReceivedFolderPath!, select: false);
                  } else {
                    _bridge.openDownloadsFolder();
                  }
                  _closePanelToIdle();
                },
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: Colors.white,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: const Text(
                    'CARPETA',
                    style: TextStyle(
                      color: Color(0xFF09090B),
                      fontSize: 10,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0.9,
                    ),
                  ),
                ),
              ),
            ),
          ] else ...[
            MouseRegion(
              cursor: SystemMouseCursors.click,
              child: GestureDetector(
                onTap: () {
                  SoundEngine.playClick(bridge: _bridge);
                  if (_lastReceivedFilePath != null) {
                    _bridge.openPath(_lastReceivedFilePath!, select: false);
                  } else {
                    _bridge.openDownloadsFolder();
                  }
                  _closePanelToIdle();
                },
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: Colors.white,
                    borderRadius: BorderRadius.circular(6),
                  ),
                  child: const Text(
                    'ABRIR',
                    style: TextStyle(
                      color: Color(0xFF09090B),
                      fontSize: 10,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0.9,
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(width: 6),
            MouseRegion(
              cursor: SystemMouseCursors.click,
              child: GestureDetector(
                onTap: () {
                  SoundEngine.playClick(bridge: _bridge);
                  if (_lastReceivedFilePath != null) {
                    _bridge.openPath(_lastReceivedFilePath!, select: true);
                  } else if (_lastReceivedFolderPath != null) {
                    _bridge.openPath(_lastReceivedFolderPath!, select: false);
                  } else {
                    _bridge.openDownloadsFolder();
                  }
                  _closePanelToIdle();
                },
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 6,
                  ),
                  decoration: BoxDecoration(
                    color: const Color(0xFF141418),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: Colors.white.withValues(alpha: 0.20),
                    ),
                  ),
                  child: const Text(
                    'CARPETA',
                    style: TextStyle(
                      color: Color(0xFFE4E4E7),
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 0.9,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ],
      ],
    );
  }
}

/// Custom geometric vector icon widget that draws directly onto the Canvas,
/// avoiding OpenGLESSDF font glyph bugs and ensuring razor-sharp Nothing OS lines.
class _MaxVectorIcon extends StatelessWidget {
  final MaxIconType type;
  final double size;
  final Color color;

  const _MaxVectorIcon({
    required this.type,
    required this.size,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: size,
      height: size,
      child: CustomPaint(
        painter: _MaxVectorIconPainter(type: type, color: color),
      ),
    );
  }
}

class _MaxVectorIconPainter extends CustomPainter {
  final MaxIconType type;
  final Color color;

  _MaxVectorIconPainter({required this.type, required this.color});

  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;
    final stroke = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.65
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;

    switch (type) {
      case MaxIconType.phone:
        final rect = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.24, h * 0.10, w * 0.52, h * 0.80),
          const Radius.circular(2.5),
        );
        canvas.drawRRect(rect, stroke);
        canvas.drawLine(
          Offset(w * 0.42, h * 0.76),
          Offset(w * 0.58, h * 0.76),
          stroke,
        );
        break;

      case MaxIconType.laptop:
        final screen = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.16, h * 0.18, w * 0.68, h * 0.48),
          const Radius.circular(2.0),
        );
        canvas.drawRRect(screen, stroke);
        canvas.drawLine(
          Offset(w * 0.08, h * 0.78),
          Offset(w * 0.92, h * 0.78),
          stroke,
        );
        break;

      case MaxIconType.tablet:
        final tab = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.18, h * 0.12, w * 0.64, h * 0.76),
          const Radius.circular(2.5),
        );
        canvas.drawRRect(tab, stroke);
        break;

      case MaxIconType.radar:
        final fill = Paint()
          ..color = color
          ..style = PaintingStyle.fill;
        canvas.drawCircle(Offset(w * 0.5, h * 0.5), w * 0.12, fill);
        canvas.drawCircle(Offset(w * 0.5, h * 0.5), w * 0.36, stroke);
        break;

      case MaxIconType.file:
        final path = Path()
          ..moveTo(w * 0.22, h * 0.12)
          ..lineTo(w * 0.58, h * 0.12)
          ..lineTo(w * 0.78, h * 0.32)
          ..lineTo(w * 0.78, h * 0.88)
          ..lineTo(w * 0.22, h * 0.88)
          ..close();
        canvas.drawPath(path, stroke);
        break;

      case MaxIconType.filePdf:
        final path = Path()
          ..moveTo(w * 0.20, h * 0.12)
          ..lineTo(w * 0.58, h * 0.12)
          ..lineTo(w * 0.80, h * 0.32)
          ..lineTo(w * 0.80, h * 0.88)
          ..lineTo(w * 0.20, h * 0.88)
          ..close();
        canvas.drawPath(path, stroke);
        canvas.drawLine(
          Offset(w * 0.33, h * 0.48),
          Offset(w * 0.67, h * 0.48),
          stroke,
        );
        canvas.drawLine(
          Offset(w * 0.33, h * 0.66),
          Offset(w * 0.54, h * 0.66),
          stroke,
        );
        break;

      case MaxIconType.fileText:
        final rect = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.20, h * 0.12, w * 0.60, h * 0.76),
          const Radius.circular(2.0),
        );
        canvas.drawRRect(rect, stroke);
        canvas.drawLine(
          Offset(w * 0.33, h * 0.36),
          Offset(w * 0.67, h * 0.36),
          stroke,
        );
        canvas.drawLine(
          Offset(w * 0.33, h * 0.52),
          Offset(w * 0.67, h * 0.52),
          stroke,
        );
        canvas.drawLine(
          Offset(w * 0.33, h * 0.68),
          Offset(w * 0.52, h * 0.68),
          stroke,
        );
        break;

      case MaxIconType.fileImage:
        final frame = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.16, h * 0.16, w * 0.68, h * 0.68),
          const Radius.circular(2.5),
        );
        canvas.drawRRect(frame, stroke);
        final mtn = Path()
          ..moveTo(w * 0.24, h * 0.72)
          ..lineTo(w * 0.44, h * 0.48)
          ..lineTo(w * 0.58, h * 0.62)
          ..lineTo(w * 0.68, h * 0.52)
          ..lineTo(w * 0.76, h * 0.72);
        canvas.drawPath(mtn, stroke);
        break;

      case MaxIconType.fileMedia:
        final box = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.16, h * 0.18, w * 0.68, h * 0.64),
          const Radius.circular(2.5),
        );
        canvas.drawRRect(box, stroke);
        final play = Path()
          ..moveTo(w * 0.42, h * 0.36)
          ..lineTo(w * 0.64, h * 0.50)
          ..lineTo(w * 0.42, h * 0.64)
          ..close();
        canvas.drawPath(play, stroke);
        break;

      case MaxIconType.fileCode:
        final leftBracket = Path()
          ..moveTo(w * 0.38, h * 0.28)
          ..lineTo(w * 0.18, h * 0.50)
          ..lineTo(w * 0.38, h * 0.72);
        final rightBracket = Path()
          ..moveTo(w * 0.62, h * 0.28)
          ..lineTo(w * 0.82, h * 0.50)
          ..lineTo(w * 0.62, h * 0.72);
        canvas.drawPath(leftBracket, stroke);
        canvas.drawPath(rightBracket, stroke);
        break;

      case MaxIconType.fileArchive:
        final box = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.18, h * 0.16, w * 0.64, h * 0.68),
          const Radius.circular(2.2),
        );
        canvas.drawRRect(box, stroke);
        canvas.drawLine(
          Offset(w * 0.50, h * 0.16),
          Offset(w * 0.50, h * 0.58),
          stroke,
        );
        canvas.drawLine(
          Offset(w * 0.40, h * 0.58),
          Offset(w * 0.60, h * 0.58),
          stroke,
        );
        break;

      case MaxIconType.plus:
        canvas.drawLine(
          Offset(w * 0.5, h * 0.18),
          Offset(w * 0.5, h * 0.82),
          stroke,
        );
        canvas.drawLine(
          Offset(w * 0.18, h * 0.5),
          Offset(w * 0.82, h * 0.5),
          stroke,
        );
        break;

      case MaxIconType.close:
        canvas.drawLine(
          Offset(w * 0.22, h * 0.22),
          Offset(w * 0.78, h * 0.78),
          stroke,
        );
        canvas.drawLine(
          Offset(w * 0.78, h * 0.22),
          Offset(w * 0.22, h * 0.78),
          stroke,
        );
        break;

      case MaxIconType.arrowUpRight:
        canvas.drawLine(
          Offset(w * 0.22, h * 0.78),
          Offset(w * 0.78, h * 0.22),
          stroke,
        );
        final head = Path()
          ..moveTo(w * 0.36, h * 0.22)
          ..lineTo(w * 0.78, h * 0.22)
          ..lineTo(w * 0.78, h * 0.64);
        canvas.drawPath(head, stroke);
        break;

      case MaxIconType.arrowDownLeft:
        canvas.drawLine(
          Offset(w * 0.78, h * 0.22),
          Offset(w * 0.22, h * 0.78),
          stroke,
        );
        final head = Path()
          ..moveTo(w * 0.22, h * 0.36)
          ..lineTo(w * 0.22, h * 0.78)
          ..lineTo(w * 0.64, h * 0.78);
        canvas.drawPath(head, stroke);
        break;

      case MaxIconType.check:
        stroke.strokeWidth = 2.1;
        final path = Path()
          ..moveTo(w * 0.18, h * 0.52)
          ..lineTo(w * 0.42, h * 0.76)
          ..lineTo(w * 0.82, h * 0.26);
        canvas.drawPath(path, stroke);
        break;

      case MaxIconType.error:
        stroke.strokeWidth = 2.0;
        canvas.drawLine(
          Offset(w * 0.24, h * 0.24),
          Offset(w * 0.76, h * 0.76),
          stroke,
        );
        canvas.drawLine(
          Offset(w * 0.76, h * 0.24),
          Offset(w * 0.24, h * 0.76),
          stroke,
        );
        break;
    }
  }

  @override
  bool shouldRepaint(covariant _MaxVectorIconPainter oldDelegate) =>
      oldDelegate.type != type || oldDelegate.color != color;
}

/// Paints a subtle Nothing OS dot-matrix background grid inside the empty drop zone.
class _NothingDotMatrixPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = Colors.white.withValues(alpha: 0.08)
      ..style = PaintingStyle.fill;

    const double step = 10.0;
    for (double x = 8.0; x < size.width - 4.0; x += step) {
      for (double y = 8.0; y < size.height - 4.0; y += step) {
        canvas.drawCircle(Offset(x, y), 0.85, paint);
      }
    }
  }

  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}

class _StagedFileCard extends StatefulWidget {
  final StagedFileItem item;
  final String formattedSize;
  final ValueChanged<bool> onHoverChanged;
  final VoidCallback onRemoveStart;
  final VoidCallback onRemove;

  const _StagedFileCard({
    super.key,
    required this.item,
    required this.formattedSize,
    required this.onHoverChanged,
    required this.onRemoveStart,
    required this.onRemove,
  });

  @override
  State<_StagedFileCard> createState() => _StagedFileCardState();
}

class _StagedFileCardState extends State<_StagedFileCard>
    with TickerProviderStateMixin {
  late final AnimationController _enterController;
  late final Animation<Offset> _slideAnimation;
  late final Animation<double> _fadeAnimation;
  late final Animation<double> _enterScaleAnimation;

  late final AnimationController _removeController;
  late final Animation<double> _exitFadeAnimation;
  late final Animation<double> _exitScaleAnimation;
  late final Animation<Offset> _exitSlideAnimation;
  late final Animation<double> _exitWidthAnimation;

  bool _isHovered = false;
  bool _isRemoving = false;

  @override
  void initState() {
    super.initState();
    _enterController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 180),
    );
    _slideAnimation = Tween<Offset>(
      begin: const Offset(0.12, 0.0),
      end: Offset.zero,
    ).animate(
      CurvedAnimation(
        parent: _enterController,
        curve: Curves.easeOutQuart,
      ),
    );
    _fadeAnimation = CurvedAnimation(
      parent: _enterController,
      curve: Curves.easeOut,
    );
    _enterScaleAnimation = Tween<double>(
      begin: 0.95,
      end: 1.0,
    ).animate(
      CurvedAnimation(
        parent: _enterController,
        curve: Curves.easeOutCubic,
      ),
    );

    _removeController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 175),
    );
    _exitFadeAnimation = Tween<double>(
      begin: 1.0,
      end: 0.0,
    ).animate(
      CurvedAnimation(
        parent: _removeController,
        curve: const Interval(0.0, 0.80, curve: Curves.easeOutCubic),
      ),
    );
    _exitScaleAnimation = Tween<double>(
      begin: 1.0,
      end: 0.92,
    ).animate(
      CurvedAnimation(
        parent: _removeController,
        curve: const Interval(0.0, 0.85, curve: Curves.easeOutCubic),
      ),
    );
    _exitSlideAnimation = Tween<Offset>(
      begin: Offset.zero,
      end: const Offset(0.0, -0.04),
    ).animate(
      CurvedAnimation(
        parent: _removeController,
        curve: const Interval(0.0, 0.85, curve: Curves.easeOutCubic),
      ),
    );
    _exitWidthAnimation = Tween<double>(
      begin: 1.0,
      end: 0.0,
    ).animate(
      CurvedAnimation(
        parent: _removeController,
        curve: const Interval(0.12, 1.0, curve: Curves.easeInOutCubic),
      ),
    );

    _enterController.forward();
  }

  Future<void> _handleRemoveTap() async {
    if (_isRemoving) return;
    setState(() {
      _isRemoving = true;
      _isHovered = false;
    });
    widget.onRemoveStart();
    await _removeController.forward();
    if (mounted) {
      widget.onRemove();
    }
  }

  @override
  void dispose() {
    _enterController.dispose();
    _removeController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final visual = _FileTypeVisual.fromFileName(widget.item.name);

    return SizeTransition(
      axis: Axis.horizontal,
      alignment: Alignment.center,
      sizeFactor: _exitWidthAnimation,
      child: FadeTransition(
        opacity: _fadeAnimation,
        child: SlideTransition(
          position: _slideAnimation,
          child: ScaleTransition(
            scale: _enterScaleAnimation,
            child: FadeTransition(
              opacity: _exitFadeAnimation,
              child: SlideTransition(
                position: _exitSlideAnimation,
                child: ScaleTransition(
                  scale: _exitScaleAnimation,
                  child: Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: MouseRegion(
                      onEnter: (_) {
                        if (!_isRemoving) {
                          setState(() => _isHovered = true);
                          widget.onHoverChanged(true);
                        }
                      },
                      onExit: (_) {
                        if (!_isRemoving) {
                          setState(() => _isHovered = false);
                          widget.onHoverChanged(false);
                        }
                      },
                      child: AnimatedContainer(
                        duration: const Duration(milliseconds: 150),
                        curve: Curves.easeOutQuart,
                        width: 162,
                        padding: const EdgeInsets.symmetric(
                          horizontal: 9,
                          vertical: 8,
                        ),
                        decoration: BoxDecoration(
                          color: _isHovered
                              ? const Color(0xFF18181D)
                              : const Color(0xFF141418),
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(
                            color: _isHovered
                                ? visual.accentColor.withValues(alpha: 0.44)
                                : visual.accentColor.withValues(alpha: 0.18),
                          ),
                        ),
                        child: Row(
                          children: [
                            // Subtle vertical accent bar for the file type
                            Container(
                              width: 2.5,
                              height: 28,
                              decoration: BoxDecoration(
                                color: visual.accentColor.withValues(
                                  alpha: _isHovered ? 0.85 : 0.48,
                                ),
                                borderRadius: BorderRadius.circular(99),
                              ),
                            ),
                            const SizedBox(width: 7),
                            Container(
                              width: 26,
                              height: 26,
                              decoration: BoxDecoration(
                                color: visual.accentColor.withValues(
                                  alpha: _isHovered ? 0.18 : 0.11,
                                ),
                                borderRadius: BorderRadius.circular(5),
                                border: Border.all(
                                  color: visual.accentColor.withValues(
                                    alpha: 0.25,
                                  ),
                                  width: 0.8,
                                ),
                              ),
                              alignment: Alignment.center,
                              child: _MaxVectorIcon(
                                type: visual.iconType,
                                size: 13,
                                color: visual.accentColor,
                              ),
                            ),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                mainAxisAlignment: MainAxisAlignment.center,
                                children: [
                                  Text(
                                    widget.item.name,
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                    style: const TextStyle(
                                      color: Colors.white,
                                      fontSize: 11,
                                      fontWeight: FontWeight.w600,
                                    ),
                                  ),
                                  const SizedBox(height: 3),
                                  Row(
                                    children: [
                                      Container(
                                        padding: const EdgeInsets.symmetric(
                                          horizontal: 4,
                                          vertical: 0.8,
                                        ),
                                        decoration: BoxDecoration(
                                          color: visual.accentColor.withValues(
                                            alpha: 0.14,
                                          ),
                                          borderRadius:
                                              BorderRadius.circular(3),
                                        ),
                                        child: Text(
                                          visual.extensionTag,
                                          style: TextStyle(
                                            color: visual.accentColor,
                                            fontSize: 8.0,
                                            fontWeight: FontWeight.w700,
                                            letterSpacing: 0.4,
                                          ),
                                        ),
                                      ),
                                      const SizedBox(width: 5),
                                      Flexible(
                                        child: Text(
                                          widget.formattedSize,
                                          maxLines: 1,
                                          overflow: TextOverflow.ellipsis,
                                          style: const TextStyle(
                                            color: Color(0xFF8E8E93),
                                            fontSize: 9.2,
                                            fontWeight: FontWeight.w500,
                                            fontFeatures: [
                                              FontFeature.tabularFigures(),
                                            ],
                                          ),
                                        ),
                                      ),
                                    ],
                                  ),
                                ],
                              ),
                            ),
                            AnimatedOpacity(
                              duration: const Duration(milliseconds: 120),
                              opacity: (_isHovered && !_isRemoving) ? 1.0 : 0.0,
                              child: IgnorePointer(
                                ignoring: !_isHovered || _isRemoving,
                                child: MouseRegion(
                                  cursor: SystemMouseCursors.click,
                                  child: GestureDetector(
                                    behavior: HitTestBehavior.opaque,
                                    onTap: _handleRemoveTap,
                                    child: Container(
                                      width: 22,
                                      height: 22,
                                      margin: const EdgeInsets.only(left: 4),
                                      decoration: BoxDecoration(
                                        color: Colors.white.withValues(
                                          alpha: 0.12,
                                        ),
                                        borderRadius: BorderRadius.circular(5),
                                      ),
                                      alignment: Alignment.center,
                                      child: const _MaxVectorIcon(
                                        type: MaxIconType.close,
                                        size: 10,
                                        color: Colors.white,
                                      ),
                                    ),
                                  ),
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
