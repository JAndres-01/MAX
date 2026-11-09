#!/usr/bin/env bash
# ========================================================
# MAX — Entorno de Desarrollo Rápido con Hot Reload (Linux)
# ========================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FLUTTER_BIN="$(which flutter 2>/dev/null || echo "$HOME/.local/share/flutter/bin/flutter")"

if [[ ! -x "$FLUTTER_BIN" ]]; then
  echo "Error: No se encontró el binario de Flutter en $HOME/.local/share/flutter/bin/flutter"
  exit 1
fi

echo "================================================="
echo "   MAX — Modo Desarrollo (Hot Reload Activo)     "
echo "================================================="
echo "Controles disponibles en la terminal:"
echo "  [r] -> Hot Reload instantáneo (~200ms) sin recompilar"
echo "  [R] -> Hot Restart completo del estado (~1s)"
echo "  [h] -> Ver lista de comandos de Flutter"
echo "  [q] -> Salir y reactivar el servicio normal"
echo "================================================="
echo "Tip: Si editas desde VS Code o Android Studio,"
echo "     al guardar (Ctrl + S) se actualiza al instante."
echo "================================================="

# Pausar temporalmente el servicio en segundo plano para no superponer la muesca
systemctl --user stop max-notch.service 2>/dev/null || true
pkill -x max_drop 2>/dev/null || true

# Asegurar que al salir se restaure el servicio de fondo
cleanup() {
  echo ""
  echo "Restaurando servicio de MAX en segundo plano..."
  systemctl --user start max-notch.service 2>/dev/null || true
}
trap cleanup EXIT INT TERM

cd "$SCRIPT_DIR/ui"
if [[ "$*" == *"--settings"* ]]; then
  "$FLUTTER_BIN" run -d linux --dart-entrypoint-args="--settings"
else
  "$FLUTTER_BIN" run -d linux "$@"
fi
