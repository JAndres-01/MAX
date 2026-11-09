import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'engine/max_engine_bridge.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(const MaxSettingsApp());
}

enum SettingsNavIconType {
  general,
  notch,
  devices,
  history,
  network,
  folder,
  trash,
  shield,
  arrowDownLeft,
  arrowUpRight,
}

class _SettingsVectorIcon extends StatelessWidget {
  final SettingsNavIconType type;
  final double size;
  final Color color;

  const _SettingsVectorIcon({
    required this.type,
    this.size = 16.0,
    required this.color,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: size,
      height: size,
      child: CustomPaint(
        painter: _SettingsVectorIconPainter(type: type, color: color),
      ),
    );
  }
}

class _SettingsVectorIconPainter extends CustomPainter {
  final SettingsNavIconType type;
  final Color color;

  _SettingsVectorIconPainter({required this.type, required this.color});

  @override
  void paint(Canvas canvas, Size size) {
    final w = size.width;
    final h = size.height;
    final stroke = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.4
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;

    final fill = Paint()
      ..color = color
      ..style = PaintingStyle.fill;

    switch (type) {
      case SettingsNavIconType.general:
        // 3 horizontal slider bars with Nothing OS indicator dots
        canvas.drawLine(Offset(w * 0.10, h * 0.26), Offset(w * 0.90, h * 0.26), stroke);
        canvas.drawCircle(Offset(w * 0.36, h * 0.26), 2.2, fill);

        canvas.drawLine(Offset(w * 0.10, h * 0.50), Offset(w * 0.90, h * 0.50), stroke);
        canvas.drawCircle(Offset(w * 0.68, h * 0.50), 2.2, fill);

        canvas.drawLine(Offset(w * 0.10, h * 0.74), Offset(w * 0.90, h * 0.74), stroke);
        canvas.drawCircle(Offset(w * 0.32, h * 0.74), 2.2, fill);
        break;

      case SettingsNavIconType.notch:
        // Stopwatch / timer circle with top button and clock hands
        canvas.drawCircle(Offset(w * 0.5, h * 0.55), w * 0.36, stroke);
        canvas.drawLine(Offset(w * 0.5, h * 0.08), Offset(w * 0.5, h * 0.18), stroke);
        canvas.drawLine(Offset(w * 0.38, h * 0.08), Offset(w * 0.62, h * 0.08), stroke);
        // Hands at 12 and 3
        canvas.drawLine(Offset(w * 0.5, h * 0.55), Offset(w * 0.5, h * 0.34), stroke);
        canvas.drawLine(Offset(w * 0.5, h * 0.55), Offset(w * 0.68, h * 0.55), stroke);
        break;

      case SettingsNavIconType.devices:
        // Silhouette duo: Desktop screen on left, smartphone on right
        final screen = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.08, h * 0.20, w * 0.52, h * 0.44),
          const Radius.circular(2.0),
        );
        canvas.drawRRect(screen, stroke);
        canvas.drawLine(Offset(w * 0.18, h * 0.74), Offset(w * 0.50, h * 0.74), stroke);

        final phone = RRect.fromRectAndRadius(
          Rect.fromLTWH(w * 0.58, h * 0.34, w * 0.34, h * 0.54),
          const Radius.circular(2.5),
        );
        canvas.drawRRect(phone, stroke);
        canvas.drawLine(Offset(w * 0.68, h * 0.80), Offset(w * 0.82, h * 0.80), stroke);
        break;

      case SettingsNavIconType.network:
        // 3 connected network hub nodes
        canvas.drawCircle(Offset(w * 0.50, h * 0.26), 2.4, fill);
        canvas.drawCircle(Offset(w * 0.22, h * 0.74), 2.2, fill);
        canvas.drawCircle(Offset(w * 0.78, h * 0.74), 2.2, fill);

        canvas.drawLine(Offset(w * 0.50, h * 0.26), Offset(w * 0.22, h * 0.74), stroke);
        canvas.drawLine(Offset(w * 0.50, h * 0.26), Offset(w * 0.78, h * 0.74), stroke);
        break;

      case SettingsNavIconType.folder:
        final path = Path()
          ..moveTo(w * 0.10, h * 0.26)
          ..lineTo(w * 0.42, h * 0.26)
          ..lineTo(w * 0.52, h * 0.36)
          ..lineTo(w * 0.90, h * 0.36)
          ..lineTo(w * 0.90, h * 0.78)
          ..lineTo(w * 0.10, h * 0.78)
          ..close();
        canvas.drawPath(path, stroke);
        break;

      case SettingsNavIconType.trash:
        // Clean X / delete
        canvas.drawLine(Offset(w * 0.22, h * 0.22), Offset(w * 0.78, h * 0.78), stroke);
        canvas.drawLine(Offset(w * 0.78, h * 0.22), Offset(w * 0.22, h * 0.78), stroke);
        break;

      case SettingsNavIconType.shield:
        final path = Path()
          ..moveTo(w * 0.50, h * 0.14)
          ..lineTo(w * 0.84, h * 0.26)
          ..lineTo(w * 0.84, h * 0.56)
          ..cubicTo(w * 0.84, h * 0.76, w * 0.50, h * 0.88, w * 0.50, h * 0.88)
          ..cubicTo(w * 0.50, h * 0.88, w * 0.16, h * 0.76, w * 0.16, h * 0.56)
          ..lineTo(w * 0.16, h * 0.26)
          ..close();
        canvas.drawPath(path, stroke);
        break;

      case SettingsNavIconType.history:
        canvas.drawArc(
          Rect.fromCircle(center: Offset(w * 0.5, h * 0.52), radius: w * 0.36),
          -2.6,
          5.2,
          false,
          stroke,
        );
        final arrowPath = Path()
          ..moveTo(w * 0.22, h * 0.18)
          ..lineTo(w * 0.16, h * 0.32)
          ..lineTo(w * 0.30, h * 0.32);
        canvas.drawPath(arrowPath, stroke);
        canvas.drawLine(Offset(w * 0.5, h * 0.52), Offset(w * 0.5, h * 0.34), stroke);
        canvas.drawLine(Offset(w * 0.5, h * 0.52), Offset(w * 0.66, h * 0.52), stroke);
        break;

      case SettingsNavIconType.arrowDownLeft:
        canvas.drawLine(Offset(w * 0.75, h * 0.25), Offset(w * 0.25, h * 0.75), stroke);
        canvas.drawLine(Offset(w * 0.25, h * 0.75), Offset(w * 0.60, h * 0.75), stroke);
        canvas.drawLine(Offset(w * 0.25, h * 0.75), Offset(w * 0.25, h * 0.40), stroke);
        break;

      case SettingsNavIconType.arrowUpRight:
        canvas.drawLine(Offset(w * 0.25, h * 0.75), Offset(w * 0.75, h * 0.25), stroke);
        canvas.drawLine(Offset(w * 0.75, h * 0.25), Offset(w * 0.40, h * 0.25), stroke);
        canvas.drawLine(Offset(w * 0.75, h * 0.25), Offset(w * 0.75, h * 0.60), stroke);
        break;
    }
  }

  @override
  bool shouldRepaint(covariant _SettingsVectorIconPainter oldDelegate) {
    return oldDelegate.type != type || oldDelegate.color != color;
  }
}

class MaxSettingsApp extends StatelessWidget {
  const MaxSettingsApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'MAX Settings',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        brightness: Brightness.dark,
        scaffoldBackgroundColor: const Color(0xFF0D0D10),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFFFFFFFF),
          surface: Color(0xFF141418),
        ),
      ),
      home: const MaxSettingsScreen(),
    );
  }
}

class MaxSettingsScreen extends StatefulWidget {
  const MaxSettingsScreen({super.key});

  @override
  State<MaxSettingsScreen> createState() => _MaxSettingsScreenState();
}

class _MaxSettingsScreenState extends State<MaxSettingsScreen> {
  final MaxEngineBridge _bridge = MaxEngineBridge();
  AppConfigModel? _config;
  bool _isLoading = true;
  int _selectedNavIndex = 0;
  String _statusMessage = 'Sincronizado con MAX';
  Timer? _statusResetTimer;

  final TextEditingController _aliasController = TextEditingController();
  final TextEditingController _downloadDirController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _bridge.addListener(_onBridgeUpdate);
    // Crucial: initialize in settings mode (DO NOT start duplicate Rust daemon / mDNS)
    _bridge.initialize(isSettingsMode: true);
    _loadInitialConfig();
  }

  @override
  void dispose() {
    _statusResetTimer?.cancel();
    _aliasController.dispose();
    _downloadDirController.dispose();
    _bridge.removeListener(_onBridgeUpdate);
    _bridge.dispose();
    super.dispose();
  }

  void _onBridgeUpdate() {
    if (!mounted) return;
    if (_bridge.currentConfig != null && _config == null) {
      _applyLoadedConfig(_bridge.currentConfig!);
    }
    setState(() {});
  }

  Future<void> _loadInitialConfig() async {
    final cfg = await _bridge.fetchConfig();
    await _bridge.fetchHistory();
    if (!mounted) return;
    if (cfg != null) {
      _applyLoadedConfig(cfg);
    } else {
      setState(() {
        _isLoading = false;
      });
    }
  }

  void _applyLoadedConfig(AppConfigModel cfg) {
    setState(() {
      _config = cfg.clone();
      _aliasController.text = cfg.alias;
      _downloadDirController.text = cfg.downloadDir;
      _isLoading = false;
    });
  }

  Future<void> _saveConfigChanges({String? feedback}) async {
    if (_config == null) return;
    final success = await _bridge.saveConfig(_config!);
    if (!mounted) return;
    setState(() {
      _statusMessage = feedback ?? (success ? 'Cambios guardados' : 'Error al guardar');
    });
    _statusResetTimer?.cancel();
    _statusResetTimer = Timer(const Duration(seconds: 3), () {
      if (mounted) {
        setState(() {
          _statusMessage = 'Sincronizado con MAX';
        });
      }
    });
  }

  Future<void> _pickDownloadDirectory() async {
    String? selected;
    if (Platform.isLinux) {
      try {
        final res = await Process.run('zenity', [
          '--file-selection',
          '--directory',
          '--title=Seleccionar carpeta de descargas para MAX',
        ]);
        if (res.exitCode == 0) {
          final out = (res.stdout as String).trim();
          if (out.isNotEmpty && Directory(out).existsSync()) {
            selected = out;
          }
        }
      } catch (_) {}
    } else if (Platform.isWindows) {
      try {
        final res = await Process.run('powershell', [
          '-NoProfile',
          '-Command',
          r"[System.Reflection.Assembly]::LoadWithPartialName('System.windows.forms') | Out-Null; $f = New-Object System.Windows.Forms.FolderBrowserDialog; if($f.ShowDialog() -eq 'OK'){ Write-Output $f.SelectedPath }"
        ]);
        if (res.exitCode == 0) {
          final out = (res.stdout as String).trim();
          if (out.isNotEmpty && Directory(out).existsSync()) {
            selected = out;
          }
        }
      } catch (_) {}
    }

    final sel = selected;
    if (sel != null && mounted) {
      setState(() {
        _downloadDirController.text = sel;
        _config?.downloadDir = sel;
      });
      _saveConfigChanges(feedback: 'Carpeta actualizada');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0D0D10),
      body: _isLoading
          ? const Center(
              child: SizedBox(
                width: 24,
                height: 24,
                child: CircularProgressIndicator(
                  strokeWidth: 2,
                  color: Colors.white,
                ),
              ),
            )
          : Row(
              children: [
                _buildSidebar(),
                VerticalDivider(width: 1, thickness: 1, color: Colors.white.withValues(alpha: 0.08)),
                Expanded(
                  child: Column(
                    children: [
                      _buildTopHeader(),
                      Divider(height: 1, thickness: 1, color: Colors.white.withValues(alpha: 0.06)),
                      Expanded(
                        child: SingleChildScrollView(
                          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 24),
                          physics: const BouncingScrollPhysics(),
                          child: _buildCurrentTabContent(),
                        ),
                      ),
                      _buildBottomStatusBar(),
                    ],
                  ),
                ),
              ],
            ),
    );
  }

  Widget _buildSidebar() {
    final navItems = [
      {'title': 'GENERAL', 'icon': SettingsNavIconType.general},
      {'title': 'NOTCH & TIEMPOS', 'icon': SettingsNavIconType.notch},
      {'title': 'DISPOSITIVOS', 'icon': SettingsNavIconType.devices},
      {'title': 'HISTORIAL', 'icon': SettingsNavIconType.history},
      {'title': 'RED & SISTEMA', 'icon': SettingsNavIconType.network},
    ];

    return Container(
      width: 216,
      color: const Color(0xFF09090C),
      padding: const EdgeInsets.symmetric(vertical: 24, horizontal: 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 7,
                height: 7,
                decoration: const BoxDecoration(
                  color: Color(0xFFD71920), // Nothing OS signature red accent dot
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 8),
              const Text(
                'MAX',
                style: TextStyle(
                  color: Colors.white,
                  fontSize: 16,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 2.0,
                  fontFamily: 'monospace',
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            'CONFIGURACIÓN',
            style: TextStyle(
              color: Colors.white.withValues(alpha: 0.35),
              fontSize: 10,
              fontWeight: FontWeight.w500,
              letterSpacing: 1.5,
              fontFamily: 'monospace',
            ),
          ),
          const SizedBox(height: 32),
          ...List.generate(navItems.length, (index) {
            final item = navItems[index];
            final bool isSelected = _selectedNavIndex == index;
            final iconColor = isSelected ? Colors.white : Colors.white.withValues(alpha: 0.45);

            return Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: InkWell(
                onTap: () => setState(() => _selectedNavIndex = index),
                borderRadius: BorderRadius.circular(10),
                child: AnimatedContainer(
                  duration: const Duration(milliseconds: 140),
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 11),
                  decoration: BoxDecoration(
                    color: isSelected
                        ? Colors.white.withValues(alpha: 0.08)
                        : Colors.transparent,
                    borderRadius: BorderRadius.circular(10),
                    border: Border.all(
                      color: isSelected
                          ? Colors.white.withValues(alpha: 0.16)
                          : Colors.transparent,
                      width: 1,
                    ),
                  ),
                  child: Row(
                    children: [
                      _SettingsVectorIcon(
                        type: item['icon'] as SettingsNavIconType,
                        size: 16,
                        color: iconColor,
                      ),
                      const SizedBox(width: 11),
                      Text(
                        item['title'] as String,
                        style: TextStyle(
                          color: isSelected
                              ? Colors.white
                              : Colors.white.withValues(alpha: 0.45),
                          fontSize: 11,
                          fontWeight: isSelected ? FontWeight.w700 : FontWeight.w500,
                          letterSpacing: 0.8,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            );
          }),
          const Spacer(),
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: const Color(0xFF131317),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: Colors.white.withValues(alpha: 0.05)),
            ),
            child: Row(
              children: [
                _SettingsVectorIcon(
                  type: SettingsNavIconType.shield,
                  size: 13,
                  color: Colors.white.withValues(alpha: 0.45),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'LocalSend v2 Core',
                    style: TextStyle(
                      color: Colors.white.withValues(alpha: 0.4),
                      fontSize: 10,
                      fontWeight: FontWeight.w500,
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildTopHeader() {
    final titles = [
      'AJUSTES GENERALES',
      'COMPORTAMIENTO DEL NOTCH',
      'DISPOSITIVOS AUTORIZADOS',
      'HISTORIAL DE TRANSFERENCIAS',
      'RED Y SISTEMA',
    ];

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 16),
      color: const Color(0xFF0D0D10),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(
            titles[_selectedNavIndex.clamp(0, titles.length - 1)],
            style: const TextStyle(
              color: Colors.white,
              fontSize: 13,
              fontWeight: FontWeight.w700,
              letterSpacing: 1.2,
              fontFamily: 'monospace',
            ),
          ),
          InkWell(
            onTap: () => _bridge.openDownloadsFolder(),
            borderRadius: BorderRadius.circular(8),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              decoration: BoxDecoration(
                color: Colors.white.withValues(alpha: 0.05),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  _SettingsVectorIcon(
                    type: SettingsNavIconType.folder,
                    size: 13,
                    color: Colors.white.withValues(alpha: 0.8),
                  ),
                  const SizedBox(width: 7),
                  const Text(
                    'DESCARGAS',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 0.8,
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCurrentTabContent() {
    if (_config == null) return const SizedBox.shrink();

    switch (_selectedNavIndex) {
      case 0:
        return _buildGeneralTab();
      case 1:
        return _buildNotchTimingsTab();
      case 2:
        return _buildDevicesTab();
      case 3:
        return _buildHistoryTab();
      case 4:
        return _buildNetworkTab();
      default:
        return const SizedBox.shrink();
    }
  }

  Widget _buildGeneralTab() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _buildSectionCard(
          title: 'NOMBRE DEL DISPOSITIVO',
          subtitle: 'Alias visible para otros dispositivos en la red local.',
          child: TextField(
            controller: _aliasController,
            style: const TextStyle(color: Colors.white, fontSize: 13, fontFamily: 'monospace'),
            decoration: InputDecoration(
              isDense: true,
              filled: true,
              fillColor: const Color(0xFF09090C),
              contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
              enabledBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
                borderSide: BorderSide(color: Colors.white.withValues(alpha: 0.1)),
              ),
              focusedBorder: OutlineInputBorder(
                borderRadius: BorderRadius.circular(8),
                borderSide: const BorderSide(color: Colors.white, width: 1.2),
              ),
            ),
            onSubmitted: (val) {
              if (val.trim().isNotEmpty) {
                _config?.alias = val.trim();
                _saveConfigChanges(feedback: 'Alias actualizado');
              }
            },
          ),
        ),
        const SizedBox(height: 20),
        _buildSectionCard(
          title: 'CARPETA DE DESCARGAS',
          subtitle: 'Directorio donde se guardan los archivos recibidos.',
          child: Row(
            children: [
              Expanded(
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                  decoration: BoxDecoration(
                    color: const Color(0xFF09090C),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
                  ),
                  child: Text(
                    _config!.downloadDir,
                    style: TextStyle(
                      color: Colors.white.withValues(alpha: 0.85),
                      fontSize: 12,
                      fontFamily: 'monospace',
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
              const SizedBox(width: 10),
              InkWell(
                onTap: _pickDownloadDirectory,
                borderRadius: BorderRadius.circular(8),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 11),
                  decoration: BoxDecoration(
                    color: Colors.white,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Text(
                    'CAMBIAR',
                    style: TextStyle(
                      color: Colors.black,
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0.5,
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 20),
        _buildSectionCard(
          title: 'EFECTOS DE SONIDO',
          subtitle: 'Feedback acústico táctil Nothing OS en transferencias y acciones.',
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                _config!.soundEffects ? 'ACTIVADO' : 'DESACTIVADO',
                style: TextStyle(
                  color: _config!.soundEffects ? Colors.white : Colors.white.withValues(alpha: 0.4),
                  fontSize: 12,
                  fontWeight: FontWeight.w700,
                  fontFamily: 'monospace',
                  letterSpacing: 1.0,
                ),
              ),
              Switch(
                value: _config!.soundEffects,
                activeThumbColor: Colors.white,
                activeTrackColor: const Color(0xFF2B2B33),
                inactiveThumbColor: Colors.white.withValues(alpha: 0.4),
                inactiveTrackColor: const Color(0xFF141418),
                onChanged: (val) {
                  setState(() {
                    _config!.soundEffects = val;
                  });
                  _saveConfigChanges(feedback: val ? 'Sonidos activados' : 'Sonidos desactivados');
                },
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildNotchTimingsTab() {
    final hoverSeconds = (_config!.hoverDelayMs / 1000.0).toStringAsFixed(1);
    final autoHideSeconds = (_config!.autoHideDelayMs / 1000.0).toStringAsFixed(2);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _buildSectionCard(
          title: 'ACTIVACIÓN AL PASAR EL CURSOR',
          subtitle: 'Tiempo que el cursor debe permanecer sobre la barra para abrir el Notch.',
          badge: '${hoverSeconds}s',
          child: SliderTheme(
            data: _sliderTheme(),
            child: Slider(
              value: _config!.hoverDelayMs.toDouble().clamp(200.0, 2500.0),
              min: 200,
              max: 2500,
              divisions: 23,
              onChanged: (val) {
                setState(() {
                  _config!.hoverDelayMs = val.round();
                });
              },
              onChangeEnd: (_) {
                _saveConfigChanges(feedback: 'Tiempo de hover actualizado');
              },
            ),
          ),
        ),
        const SizedBox(height: 20),
        _buildSectionCard(
          title: 'TIEMPO DE AUTO-OCULTADO',
          subtitle: 'Espera antes de replegar el Notch al retirar el cursor del área.',
          badge: '${autoHideSeconds}s',
          child: SliderTheme(
            data: _sliderTheme(),
            child: Slider(
              value: _config!.autoHideDelayMs.toDouble().clamp(100.0, 1500.0),
              min: 100,
              max: 1500,
              divisions: 14,
              onChanged: (val) {
                setState(() {
                  _config!.autoHideDelayMs = val.round();
                });
              },
              onChangeEnd: (_) {
                _saveConfigChanges(feedback: 'Tiempo de ocultado actualizado');
              },
            ),
          ),
        ),
        const SizedBox(height: 20),
        _buildSectionCard(
          title: 'ANCHO DEL NOTCH EN REPOSO',
          subtitle: 'Dimensión horizontal de la barra superior cuando no está activa.',
          badge: '${_config!.notchIdleWidth.round()} px',
          child: SliderTheme(
            data: _sliderTheme(),
            child: Slider(
              value: _config!.notchIdleWidth.clamp(100.0, 220.0),
              min: 100,
              max: 220,
              divisions: 12,
              onChanged: (val) {
                setState(() {
                  _config!.notchIdleWidth = val;
                });
              },
              onChangeEnd: (_) {
                _saveConfigChanges(feedback: 'Ancho en reposo actualizado');
              },
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildDevicesTab() {
    final trustedList = _config!.trustedDevices
        .where((item) => !(item.length == 64 && RegExp(r'^[0-9a-fA-F]+$').hasMatch(item)))
        .toList();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _buildSectionCard(
          title: 'ACEPTAR DE CUALQUIER DISPOSITIVO',
          subtitle: 'Recibir transferencias entrantes sin requerir confirmación manual.',
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                _config!.autoAccept ? 'ACTIVADO' : 'DESACTIVADO',
                style: TextStyle(
                  color: _config!.autoAccept ? Colors.white : Colors.white.withValues(alpha: 0.4),
                  fontSize: 12,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 0.8,
                  fontFamily: 'monospace',
                ),
              ),
              Switch(
                value: _config!.autoAccept,
                activeThumbColor: Colors.white,
                activeTrackColor: Colors.white.withValues(alpha: 0.3),
                inactiveThumbColor: Colors.white.withValues(alpha: 0.4),
                inactiveTrackColor: Colors.white.withValues(alpha: 0.1),
                onChanged: (val) {
                  setState(() {
                    _config!.autoAccept = val;
                  });
                  _saveConfigChanges(feedback: val ? 'Auto-aceptar activado' : 'Auto-aceptar desactivado');
                },
              ),
            ],
          ),
        ),
        const SizedBox(height: 20),
        _buildSectionCard(
          title: 'DISPOSITIVOS DE CONFIANZA (WHITELIST)',
          subtitle: 'Dispositivos recordados con autorización permanente.',
          child: trustedList.isEmpty
              ? Container(
                  width: double.infinity,
                  padding: const EdgeInsets.symmetric(vertical: 24),
                  alignment: Alignment.center,
                  child: Text(
                    'No hay dispositivos autorizados en la lista.',
                    style: TextStyle(
                      color: Colors.white.withValues(alpha: 0.35),
                      fontSize: 11,
                      fontFamily: 'monospace',
                    ),
                  ),
                )
              : Column(
                  children: trustedList.map((item) {
                    return Container(
                      margin: const EdgeInsets.only(bottom: 8),
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                      decoration: BoxDecoration(
                        color: const Color(0xFF09090C),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
                      ),
                      child: Row(
                        children: [
                          _SettingsVectorIcon(
                            type: SettingsNavIconType.shield,
                            size: 13,
                            color: Colors.white.withValues(alpha: 0.7),
                          ),
                          const SizedBox(width: 10),
                          Expanded(
                            child: Text(
                              item,
                              style: const TextStyle(
                                color: Colors.white,
                                fontSize: 12,
                                fontWeight: FontWeight.w500,
                                fontFamily: 'monospace',
                              ),
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          InkWell(
                            onTap: () async {
                              await _bridge.removeTrustedDevice(item);
                              setState(() {
                                _config?.trustedDevices.remove(item);
                              });
                              _saveConfigChanges(feedback: 'Dispositivo eliminado');
                            },
                            borderRadius: BorderRadius.circular(6),
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                              decoration: BoxDecoration(
                                color: const Color(0xFFD71920).withValues(alpha: 0.15),
                                borderRadius: BorderRadius.circular(6),
                                border: Border.all(color: const Color(0xFFD71920).withValues(alpha: 0.4)),
                              ),
                              child: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  _SettingsVectorIcon(
                                    type: SettingsNavIconType.trash,
                                    size: 10,
                                    color: const Color(0xFFFF4D4D),
                                  ),
                                  const SizedBox(width: 5),
                                  const Text(
                                    'ELIMINAR',
                                    style: TextStyle(
                                      color: Color(0xFFFF4D4D),
                                      fontSize: 10,
                                      fontWeight: FontWeight.w700,
                                      letterSpacing: 0.5,
                                      fontFamily: 'monospace',
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ],
                      ),
                    );
                  }).toList(),
                ),
        ),
      ],
    );
  }

  Widget _buildHistoryTab() {
    final history = _bridge.history;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          mainAxisAlignment: MainAxisAlignment.spaceBetween,
          children: [
            Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const Text(
                  'REGISTRO DE TRANSFERENCIAS',
                  style: TextStyle(
                    color: Colors.white,
                    fontSize: 12,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 1.0,
                    fontFamily: 'monospace',
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  history.isEmpty
                      ? 'Sin actividad reciente'
                      : '${history.length} ${history.length == 1 ? 'elemento registrado' : 'elementos registrados'}',
                  style: TextStyle(
                    color: Colors.white.withValues(alpha: 0.4),
                    fontSize: 11,
                  ),
                ),
              ],
            ),
            if (history.isNotEmpty)
              InkWell(
                onTap: () async {
                  await _bridge.clearHistory();
                  setState(() {});
                },
                borderRadius: BorderRadius.circular(7),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 11, vertical: 6),
                  decoration: BoxDecoration(
                    color: const Color(0xFF1D1415),
                    borderRadius: BorderRadius.circular(7),
                    border: Border.all(
                      color: const Color(0xFFD71920).withValues(alpha: 0.4),
                    ),
                  ),
                  child: const Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      _SettingsVectorIcon(
                        type: SettingsNavIconType.trash,
                        size: 11,
                        color: Color(0xFFD71920),
                      ),
                      SizedBox(width: 6),
                      Text(
                        'BORRAR TODO',
                        style: TextStyle(
                          color: Color(0xFFD71920),
                          fontSize: 10,
                          fontWeight: FontWeight.w700,
                          letterSpacing: 0.8,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                ),
              ),
          ],
        ),
        const SizedBox(height: 16),
        if (history.isEmpty)
          Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(vertical: 48, horizontal: 24),
            decoration: BoxDecoration(
              color: const Color(0xFF141418),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: Colors.white.withValues(alpha: 0.06)),
            ),
            alignment: Alignment.center,
            child: Column(
              children: [
                _SettingsVectorIcon(
                  type: SettingsNavIconType.history,
                  size: 32,
                  color: Colors.white.withValues(alpha: 0.22),
                ),
                const SizedBox(height: 14),
                Text(
                  'NO HAY TRANSFERENCIAS REGISTRADAS',
                  style: TextStyle(
                    color: Colors.white.withValues(alpha: 0.45),
                    fontSize: 11,
                    fontWeight: FontWeight.w700,
                    letterSpacing: 1.0,
                    fontFamily: 'monospace',
                  ),
                ),
                const SizedBox(height: 5),
                Text(
                  'Los archivos recibidos o enviados aparecerán aquí.',
                  style: TextStyle(
                    color: Colors.white.withValues(alpha: 0.25),
                    fontSize: 11,
                  ),
                ),
              ],
            ),
          )
        else
          Column(
            children: history.map((item) {
              final isIncoming = item.isIncoming;
              final accentColor = item.success
                  ? (isIncoming ? const Color(0xFF34D399) : Colors.white)
                  : const Color(0xFFD71920);

              return Container(
                margin: const EdgeInsets.only(bottom: 8),
                padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                decoration: BoxDecoration(
                  color: const Color(0xFF141418),
                  borderRadius: BorderRadius.circular(10),
                  border: Border.all(color: Colors.white.withValues(alpha: 0.07)),
                ),
                child: Row(
                  children: [
                    Container(
                      width: 28,
                      height: 28,
                      decoration: BoxDecoration(
                        color: const Color(0xFF0D0D10),
                        borderRadius: BorderRadius.circular(7),
                        border: Border.all(
                          color: accentColor.withValues(alpha: 0.25),
                        ),
                      ),
                      alignment: Alignment.center,
                      child: _SettingsVectorIcon(
                        type: isIncoming
                            ? SettingsNavIconType.arrowDownLeft
                            : SettingsNavIconType.arrowUpRight,
                        size: 13,
                        color: accentColor,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            item.fileName,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 12,
                              fontWeight: FontWeight.w600,
                              fontFamily: 'monospace',
                            ),
                          ),
                          const SizedBox(height: 3),
                          Text(
                            '${isIncoming ? 'Recibido de' : 'Enviado a'} ${item.deviceAlias}  •  ${item.formattedSize}  •  ${item.formattedDate}',
                            style: TextStyle(
                              color: Colors.white.withValues(alpha: 0.40),
                              fontSize: 10.5,
                              letterSpacing: 0.2,
                            ),
                          ),
                        ],
                      ),
                    ),
                    if (item.filePath != null && File(item.filePath!).existsSync()) ...[
                      const SizedBox(width: 8),
                      InkWell(
                        onTap: () => _bridge.openPath(item.filePath!, select: false),
                        borderRadius: BorderRadius.circular(6),
                        child: Container(
                          padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 5),
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
                              fontFamily: 'monospace',
                              letterSpacing: 0.5,
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 6),
                      InkWell(
                        onTap: () => _bridge.openPath(item.filePath!, select: true),
                        borderRadius: BorderRadius.circular(6),
                        child: Container(
                          padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 5),
                          decoration: BoxDecoration(
                            color: const Color(0xFF0D0D10),
                            borderRadius: BorderRadius.circular(6),
                            border: Border.all(color: Colors.white.withValues(alpha: 0.15)),
                          ),
                          child: Text(
                            'CARPETA',
                            style: TextStyle(
                              color: Colors.white.withValues(alpha: 0.8),
                              fontSize: 10,
                              fontWeight: FontWeight.w600,
                              fontFamily: 'monospace',
                              letterSpacing: 0.5,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
              );
            }).toList(),
          ),
      ],
    );
  }

  Widget _buildNetworkTab() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        _buildSectionCard(
          title: 'INICIO CON EL SISTEMA',
          subtitle: 'Iniciar MAX Notch automáticamente al encender el equipo.',
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                _config!.autostart ? 'ACTIVADO' : 'DESACTIVADO',
                style: TextStyle(
                  color: _config!.autostart ? Colors.white : Colors.white.withValues(alpha: 0.4),
                  fontSize: 12,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 0.8,
                  fontFamily: 'monospace',
                ),
              ),
              Switch(
                value: _config!.autostart,
                activeThumbColor: Colors.white,
                activeTrackColor: Colors.white.withValues(alpha: 0.3),
                inactiveThumbColor: Colors.white.withValues(alpha: 0.4),
                inactiveTrackColor: Colors.white.withValues(alpha: 0.1),
                onChanged: (val) {
                  setState(() {
                    _config!.autostart = val;
                  });
                  _saveConfigChanges(feedback: val ? 'Inicio automático activado' : 'Inicio automático desactivado');
                },
              ),
            ],
          ),
        ),
        const SizedBox(height: 20),
        _buildSectionCard(
          title: 'PUERTO LOCAL',
          subtitle: 'Puerto estándar utilizado por el protocolo LocalSend v2.',
          badge: '${_config!.port}',
          child: Container(
            width: double.infinity,
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
            decoration: BoxDecoration(
              color: const Color(0xFF09090C),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: Colors.white.withValues(alpha: 0.08)),
            ),
            child: Text(
              'Puerto 53317 (Permitido en cortafuegos UFW y Windows Firewall)',
              style: TextStyle(
                color: Colors.white.withValues(alpha: 0.6),
                fontSize: 11,
                fontFamily: 'monospace',
              ),
            ),
          ),
        ),
        const SizedBox(height: 20),
        _buildSectionCard(
          title: 'PROTOCOLO DE SEGURIDAD',
          subtitle: 'Cifrado de transferencias mediante TLS autogenerado.',
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                _config!.protocol.toUpperCase() == 'HTTPS'
                    ? 'HTTPS (Cifrado TLS Activo)'
                    : 'HTTP (Texto plano)',
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  fontFamily: 'monospace',
                ),
              ),
              InkWell(
                onTap: () {
                  final newProto = _config!.protocol == 'https' ? 'http' : 'https';
                  setState(() {
                    _config!.protocol = newProto;
                  });
                  _saveConfigChanges(feedback: 'Protocolo cambiado a $newProto');
                },
                borderRadius: BorderRadius.circular(6),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                  decoration: BoxDecoration(
                    color: Colors.white.withValues(alpha: 0.06),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.12)),
                  ),
                  child: const Text(
                    'CAMBIAR',
                    style: TextStyle(
                      color: Colors.white,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 0.5,
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildSectionCard({
    required String title,
    required String subtitle,
    String? badge,
    required Widget child,
  }) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        color: const Color(0xFF141418),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.white.withValues(alpha: 0.06)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text(
                title,
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 11,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 1.0,
                  fontFamily: 'monospace',
                ),
              ),
              if (badge != null)
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: Colors.white.withValues(alpha: 0.08),
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
                  ),
                  child: Text(
                    badge,
                    style: const TextStyle(
                      color: Colors.white,
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            subtitle,
            style: TextStyle(
              color: Colors.white.withValues(alpha: 0.40),
              fontSize: 11,
              height: 1.3,
            ),
          ),
          const SizedBox(height: 14),
          child,
        ],
      ),
    );
  }

  SliderThemeData _sliderTheme() {
    return SliderThemeData(
      activeTrackColor: Colors.white,
      inactiveTrackColor: Colors.white.withValues(alpha: 0.12),
      thumbColor: Colors.white,
      overlayColor: Colors.white.withValues(alpha: 0.08),
      thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
      trackHeight: 3,
    );
  }

  Widget _buildBottomStatusBar() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 12),
      color: const Color(0xFF09090C),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Row(
            children: [
              Container(
                width: 6,
                height: 6,
                decoration: const BoxDecoration(
                  color: Color(0xFF55FF88),
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                _statusMessage,
                style: TextStyle(
                  color: Colors.white.withValues(alpha: 0.5),
                  fontSize: 10,
                  fontWeight: FontWeight.w500,
                  letterSpacing: 0.5,
                  fontFamily: 'monospace',
                ),
              ),
            ],
          ),
          Text(
            'MAX v1.0.0',
            style: TextStyle(
              color: Colors.white.withValues(alpha: 0.25),
              fontSize: 10,
              fontFamily: 'monospace',
            ),
          ),
        ],
      ),
    );
  }
}
