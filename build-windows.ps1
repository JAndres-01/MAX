# Script de compilación y empaquetado de MAX para Windows (x64)
$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "   Compilación y Empaquetado de MAX       " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# 1. Compilar motor Rust embebido (max_core.dll)
Write-Host "[1/4] Compilando motor Rust embebido (max_core.dll)..." -ForegroundColor Yellow
Push-Location "$PSScriptRoot\core"
cargo build --release --lib
Pop-Location

# 2. Compilar interfaz Flutter Desktop
Write-Host "[2/4] Compilando interfaz Flutter Desktop (Windows Release)..." -ForegroundColor Yellow
Push-Location "$PSScriptRoot\ui"
flutter build windows --release
Pop-Location

# 3. Ensamblar directorio Release
Write-Host "[3/4] Ensamblando bundle de distribución..." -ForegroundColor Yellow
$BundleDir = "$PSScriptRoot\ui\build\windows\x64\runner\Release"
Copy-Item -Force "$PSScriptRoot\core\target\release\max_core.dll" "$BundleDir\max_core.dll"
Write-Host "Ejecutable y dependencias listos en: $BundleDir\max_drop.exe" -ForegroundColor Green

# 4. Generar instalador autónomo (.exe) con Inno Setup si está instalado
Write-Host "[4/4] Buscando Inno Setup Compiler para generar MAX-Setup-x64.exe..." -ForegroundColor Yellow

$IsccPaths = @(
    "iscc",
    "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
    "C:\Program Files\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
)

$IsccExe = $null
foreach ($path in $IsccPaths) {
    if (Get-Command $path -ErrorAction SilentlyContinue) {
        $IsccExe = $path
        break
    } elseif (Test-Path $path) {
        $IsccExe = $path
        break
    }
}

if ($IsccExe) {
    Write-Host "Compilando instalador con Inno Setup ($IsccExe)..." -ForegroundColor Cyan
    & $IsccExe "$PSScriptRoot\installer\max_setup.iss"
    Write-Host "¡INSTALADOR CREADO CON ÉXITO!" -ForegroundColor Green
    Write-Host "Archivo: $PSScriptRoot\installer\output\MAX-Setup-x64.exe" -ForegroundColor Green
} else {
    Write-Host "Inno Setup no encontrado en el sistema." -ForegroundColor DarkYellow
    Write-Host "Puedes generar el instalador .exe de 2 formas:" -ForegroundColor Gray
    Write-Host "  a) Instalando Inno Setup: winget install JRSoftware.InnoSetup y re-ejecutar este script." -ForegroundColor Gray
    Write-Host "  b) Subiendo tus cambios a GitHub: el workflow de GitHub Actions compilará y generará el instalador automáticamente." -ForegroundColor Gray
}

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "   Proceso finalizado con éxito           " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
