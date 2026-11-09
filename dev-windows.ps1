# ========================================================
# MAX — Entorno de Desarrollo Rápido con Hot Reload (Windows)
# ========================================================
$ErrorActionPreference = "Stop"

Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "   MAX — Modo Desarrollo Windows (Hot Reload)   " -ForegroundColor Cyan
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "Controles disponibles en la terminal:" -ForegroundColor Yellow
Write-Host "  [r] -> Hot Reload instantaneo (~200ms) sin recompilar" -ForegroundColor Gray
Write-Host "  [R] -> Hot Restart completo del estado (~1s)" -ForegroundColor Gray
Write-Host "  [h] -> Ver lista de comandos de Flutter" -ForegroundColor Gray
Write-Host "  [q] -> Salir" -ForegroundColor Gray
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "Tip: Si usas VS Code, al guardar (Ctrl + S) los" -ForegroundColor Green
Write-Host "     cambios visuales se reflejan de inmediato." -ForegroundColor Green
Write-Host "=================================================" -ForegroundColor Cyan

# Cerrar instancias en ejecucion de max_drop
Stop-Process -Name "max_drop" -ErrorAction SilentlyContinue

Push-Location "$PSScriptRoot\ui"
flutter run -d windows $args
Pop-Location
