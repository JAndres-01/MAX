# MAX — Top-Notch Multiplatform File Transfer (Linux & Windows)

**MAX** es una aplicación monolítica multiplataforma (**Linux** y **Windows**) de transferencia P2P ultrarrápida compatible con el protocolo **LocalSend v2.1**, diseñada bajo una estética **Glassmorphism Azul Oscuro** como un **Notch superior inteligente**.

---

## Arquitectura

- **`core/` (`max-core` — Rust):**
  - Motor asíncrono (`tokio` + `axum` + `rustls`) compilado como biblioteca dinámica nativa (`libmax_core.so` en Linux, `max_core.dll` en Windows) y binario autónomo.
  - Descubrimiento híbrido UDP Multicast (`224.0.0.167:53317`) + Keep-Alive HTTP `/register` activo con TTL de 30s para máxima estabilidad con iOS/Android.
  - Control de acceso de recepción con **bloqueo antes de escribir en disco** (`handle_prepare_upload`) y **Lista Blanca (`trusted_devices`)** persistente por huella TLS (`fingerprint`) o alias.
  - Puente de control local (`127.0.0.1:53320`) con streaming de eventos en tiempo real por WebSocket (`/api/max/v1/events`).

- **`ui/` (`max_drop` — Flutter Desktop):**
  - Carga `libmax_core.so` / `max_core.dll` directamente en memoria mediante `dart:ffi` (`max_core_start()`).
  - **Estado en reposo:** Franja luminosa cian de `3px` en el borde superior central. No se abre con clic.
  - **Apertura inteligente:**
    - Manteniendo el cursor **2.0 segundos** sobre el notch (con barra de carga luminosa progresiva).
    - **Al instante (0 ms)** al arrastrar uno o más archivos hacia el notch.
  - **Selección de dispositivos por Microdots (`[ ━ ] • •`):** Nombre del dispositivo activo en tipografía limpia acompañado de micro-indicadores interactivos.
  - **Cola persistente con animación derecha a izquierda:** Los archivos añadidos entran suavemente de derecha a izquierda y se conservan en memoria si ocultas el panel alejando el cursor.
  - **Bloqueo de visibilidad en transferencias y notificaciones:** El panel se contrae eliminando espacios vacíos y **permanece visible aunque alejes el cursor** hasta que finaliza la transferencia y la animación de confirmación.

---

## Ejecución Rápida (Linux)

```bash
chmod +x /home/hyprand/Proyectos/antigravity/MAX/run-max.sh
/home/hyprand/Proyectos/antigravity/MAX/run-max.sh
```

## Compilación y Generación del Instalador en Windows

### Opción A: Compilación en tu PC (PowerShell)
Ejecuta en PowerShell como Administrador o usuario estándar:
```powershell
.\build-windows.ps1
```
1. Compilará el motor Rust (`max_core.dll`).
2. Compilará la interfaz Flutter en modo Release.
3. Si tienes [Inno Setup](https://jrsoftware.org/isinfo.php) instalado (`winget install JRSoftware.InnoSetup`), creará automáticamente el instalador en:
   ```text
   installer\output\MAX-Setup-x64.exe
   ```

### Opción B: Compilación Automática en la Nube (GitHub Actions)
El repositorio incluye el flujo [`.github/workflows/build-windows.yml`](file:///.github/workflows/build-windows.yml):
* Puedes presionar **"Run workflow"** en la pestaña **Actions** de GitHub.
* O crear una etiqueta de versión (ej. `v1.0.0`).
* GitHub compilará todo en servidores Windows de Microsoft y te entregará el archivo **`MAX-Setup-x64.exe`** listo para descargar y compartir.

---

## Experiencia de Instalación para el Usuario Final

El instalador `MAX-Setup-x64.exe`:
* Se instala en **1 clic** (asistente en español e inglés).
* Permite instalar sin permisos de Administrador en `%LOCALAPPDATA%\Programs\MAX` o para todos los usuarios en `Program Files`.
* Añade acceso directo en el **Escritorio** y en el **Menú Inicio**.
* Ofrece casilla opcional para **iniciar MAX automáticamente al encender Windows**.
* Configura la regla de Windows Firewall para permitir transferencias P2P en redes locales sin alertas molestas.
* Proporciona un desinstalador limpio desde "Configuración > Aplicaciones instaladas" de Windows.

