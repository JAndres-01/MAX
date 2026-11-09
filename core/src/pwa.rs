pub const PWA_HTML: &str = r####"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover">
    <title>Synapse Continuity</title>
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
    <meta name="apple-mobile-web-app-title" content="Synapse">
    <meta name="theme-color" content="#08080a">
    <link rel="icon" type="image/svg+xml" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='white'><path d='M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z'/></svg>">
    <style>
        :root {
            --bg: #08080a;
            --card-bg: #121216;
            --card-hover: #18181f;
            --border: rgba(255, 255, 255, 0.1);
            --border-focus: rgba(255, 255, 255, 0.35);
            --text: #ffffff;
            --text-dim: #888894;
            --accent: #ffffff;
            --accent-green: #34d399;
            --radius-lg: 20px;
            --radius-md: 14px;
            --radius-sm: 8px;
        }

        * {
            box-sizing: border-box;
            margin: 0;
            padding: 0;
            -webkit-tap-highlight-color: transparent;
            font-family: -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Segoe UI", Roboto, sans-serif;
            user-select: none;
            -webkit-user-select: none;
        }

        body {
            background-color: var(--bg);
            color: var(--text);
            min-height: 100vh;
            padding: env(safe-area-inset-top, 20px) 16px env(safe-area-inset-bottom, 24px) 16px;
            display: flex;
            flex-direction: column;
            align-items: center;
            overflow-x: hidden;
        }

        .container {
            width: 100%;
            max-width: 440px;
            display: flex;
            flex-direction: column;
            gap: 16px;
        }

        /* Header */
        .header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 12px 4px;
        }

        .brand {
            display: flex;
            flex-direction: column;
        }

        .brand-title {
            font-size: 20px;
            font-weight: 700;
            letter-spacing: -0.5px;
            display: flex;
            align-items: center;
            gap: 6px;
        }

        .brand-sub {
            font-size: 11px;
            color: var(--text-dim);
            text-transform: uppercase;
            letter-spacing: 1px;
            margin-top: 2px;
        }

        .status-badge {
            display: flex;
            align-items: center;
            gap: 6px;
            background: rgba(52, 211, 153, 0.12);
            border: 1px solid rgba(52, 211, 153, 0.25);
            padding: 4px 10px;
            border-radius: 999px;
            font-size: 11px;
            font-weight: 600;
            color: var(--accent-green);
        }

        .status-dot {
            width: 6px;
            height: 6px;
            border-radius: 50%;
            background-color: var(--accent-green);
            box-shadow: 0 0 8px var(--accent-green);
            animation: pulse 2s infinite ease-in-out;
        }

        @keyframes pulse {
            0%, 100% { opacity: 1; transform: scale(1); }
            50% { opacity: 0.5; transform: scale(0.85); }
        }

        /* Cards */
        .card {
            background: var(--card-bg);
            border: 1px solid var(--border);
            border-radius: var(--radius-lg);
            padding: 16px;
            display: flex;
            flex-direction: column;
            gap: 12px;
            backdrop-filter: blur(20px);
            -webkit-backdrop-filter: blur(20px);
            box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
            transition: border-color 0.2s ease;
        }

        .card-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
        }

        .card-title {
            font-size: 13px;
            font-weight: 600;
            color: var(--text-dim);
            text-transform: uppercase;
            letter-spacing: 0.5px;
            display: flex;
            align-items: center;
            gap: 6px;
        }

        .card-time {
            font-size: 11px;
            color: var(--text-dim);
            font-variant-numeric: tabular-nums;
        }

        /* Clipboard Display Box */
        .clip-box {
            background: rgba(0, 0, 0, 0.35);
            border: 1px solid var(--border);
            border-radius: var(--radius-md);
            padding: 12px;
            font-size: 14px;
            line-height: 1.4;
            max-height: 140px;
            overflow-y: auto;
            word-break: break-word;
            user-select: text;
            -webkit-user-select: text;
            white-space: pre-wrap;
            font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
        }

        /* Buttons */
        .btn {
            background: #ffffff;
            color: #000000;
            border: none;
            border-radius: var(--radius-md);
            padding: 12px 16px;
            font-size: 14px;
            font-weight: 600;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 8px;
            cursor: pointer;
            transition: all 0.15s ease;
            box-shadow: 0 4px 12px rgba(255, 255, 255, 0.15);
        }

        .btn:active {
            transform: scale(0.97);
            opacity: 0.9;
        }

        .btn-secondary {
            background: rgba(255, 255, 255, 0.08);
            color: var(--text);
            border: 1px solid var(--border);
            box-shadow: none;
        }

        .btn-secondary:active {
            background: rgba(255, 255, 255, 0.14);
        }

        /* Textarea Input */
        .input-area {
            background: rgba(0, 0, 0, 0.35);
            border: 1px solid var(--border);
            border-radius: var(--radius-md);
            padding: 12px;
            font-size: 14px;
            color: var(--text);
            width: 100%;
            height: 90px;
            resize: none;
            outline: none;
            user-select: text;
            -webkit-user-select: text;
            font-family: inherit;
            transition: border-color 0.2s;
        }

        .input-area:focus {
            border-color: var(--border-focus);
        }

        .input-area::placeholder {
            color: rgba(255, 255, 255, 0.25);
        }

        /* History List */
        .history-list {
            display: flex;
            flex-direction: column;
            gap: 6px;
        }

        .history-item {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 8px 10px;
            background: rgba(255, 255, 255, 0.03);
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: var(--radius-sm);
            cursor: pointer;
            transition: background 0.15s;
        }

        .history-item:active {
            background: rgba(255, 255, 255, 0.08);
        }

        .history-text {
            font-size: 12px;
            white-space: nowrap;
            overflow: hidden;
            text-overflow: ellipsis;
            max-width: 80%;
            color: var(--text);
        }

        .history-icon {
            font-size: 11px;
            color: var(--text-dim);
        }

        /* Toast notification */
        .toast {
            position: fixed;
            bottom: 30px;
            background: #ffffff;
            color: #000000;
            padding: 10px 18px;
            border-radius: 999px;
            font-size: 13px;
            font-weight: 600;
            box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
            opacity: 0;
            transform: translateY(20px);
            transition: all 0.25s cubic-bezier(0.16, 1, 0.3, 1);
            pointer-events: none;
            z-index: 100;
        }

        .toast.show {
            opacity: 1;
            transform: translateY(0);
        }

        /* Shortcuts Guide */
        .shortcut-card {
            border: 1px dashed rgba(255, 255, 255, 0.2);
            background: rgba(255, 255, 255, 0.02);
        }
    </style>
</head>
<body>
    <div class="container">
        <!-- Header -->
        <div class="header">
            <div class="brand">
                <div class="brand-title">⚡ Synapse Continuity</div>
                <div class="brand-sub">Hyprland & iPhone Sync</div>
            </div>
            <div class="status-badge" id="statusBadge">
                <div class="status-dot"></div>
                <span id="statusText">En línea</span>
            </div>
        </div>

        <!-- Card 1: PC Clipboard -> iPhone -->
        <div class="card">
            <div class="card-header">
                <span class="card-title">🖥️ Portapapeles de la PC</span>
                <span class="card-time" id="pcClipTime">Sincronizado</span>
            </div>
            <div class="clip-box" id="pcClipText">Cargando...</div>
            <button class="btn" id="btnCopyFromPc" onclick="copyPcClipToPhone()">
                <span>📋 Copiar a este iPhone</span>
            </button>
        </div>

        <!-- Card 2: iPhone -> PC Clipboard -->
        <div class="card">
            <div class="card-header">
                <span class="card-title">📱 Enviar Texto a la PC</span>
            </div>
            <textarea class="input-area" id="phoneTextInput" placeholder="Escribe o pega texto/enlace aquí para mandarlo al portapapeles de Linux..."></textarea>
            <div style="display: flex; gap: 8px;">
                <button class="btn btn-secondary" style="flex: 1;" onclick="pasteFromPhoneClipboard()">
                    <span>📄 Pegar de iPhone</span>
                </button>
                <button class="btn" style="flex: 1.2;" onclick="sendTextToPc()">
                    <span>⚡ Enviar a PC</span>
                </button>
            </div>
        </div>

        <!-- Card 3: Photos & Files Upload -->
        <div class="card">
            <div class="card-header">
                <span class="card-title">📷 Enviar Fotos o Archivos</span>
            </div>
            <input type="file" id="fileUploadInput" multiple style="display: none;" onchange="handleFileSelection(this.files)">
            <button class="btn btn-secondary" onclick="document.getElementById('fileUploadInput').click()">
                <span>📁 Seleccionar Fotos / Documentos</span>
            </button>
            <div id="uploadStatusText" style="font-size: 11px; color: var(--text-dim); text-align: center; display: none;"></div>
        </div>

        <!-- Card 4: Apple Shortcuts -->
        <div class="card shortcut-card">
            <div class="card-header">
                <span class="card-title">⚡ Atajos de Apple (1-Tap)</span>
            </div>
            <p style="font-size: 12px; color: var(--text-dim); line-height: 1.4;">
                Sincroniza al instante tocando 2 veces la parte trasera de tu iPhone (Back Tap):
            </p>
            <div style="display: flex; flex-direction: column; gap: 6px;">
                <button class="btn btn-secondary" onclick="showShortcutGuide()">
                    <span>🛠️ Ver Receta de Atajo de iOS</span>
                </button>
            </div>
        </div>

        <!-- Card 4: Recent History -->
        <div class="card">
            <div class="card-header">
                <span class="card-title">📜 Historial Reciente</span>
            </div>
            <div class="history-list" id="historyList">
                <div style="font-size: 12px; color: var(--text-dim); text-align: center; padding: 8px;">Sin historial reciente</div>
            </div>
        </div>
    </div>

    <div class="toast" id="toast">Copiado con éxito</div>

    <script>
        let currentPcClip = "";

        function showToast(msg) {
            const toast = document.getElementById("toast");
            toast.innerText = msg;
            toast.classList.add("show");
            if (navigator.vibrate) navigator.vibrate(40);
            setTimeout(() => toast.classList.remove("show"), 2200);
        }

        async function fetchClipboard() {
            try {
                const res = await fetch("/api/clipboard");
                if (res.ok) {
                    const data = await res.json();
                    if (data.text) {
                        updatePcClipDisplay(data.text);
                    }
                }
            } catch (e) {
                console.warn("Fetch clipboard failed:", e);
            }
        }

        async function fetchHistory() {
            try {
                const res = await fetch("/api/clipboard/history");
                if (res.ok) {
                    const list = await res.json();
                    renderHistory(list);
                }
            } catch (e) {}
        }

        function updatePcClipDisplay(text) {
            currentPcClip = text;
            const el = document.getElementById("pcClipText");
            el.innerText = text;
            document.getElementById("pcClipTime").innerText = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
        }

        function renderHistory(list) {
            const container = document.getElementById("historyList");
            if (!list || list.length === 0) {
                container.innerHTML = '<div style="font-size: 12px; color: var(--text-dim); text-align: center; padding: 8px;">Sin historial reciente</div>';
                return;
            }

            container.innerHTML = list.slice(0, 5).map(item => `
                <div class="history-item" onclick="selectHistoryItem(${JSON.stringify(item.text).replace(/"/g, '&quot;')})">
                    <span class="history-text">${escapeHtml(item.text)}</span>
                    <span class="history-icon">${item.source === 'iphone' ? '📱' : '🖥️'}</span>
                </div>
            `).join('');
        }

        function escapeHtml(str) {
            return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#039;");
        }

        async function copyPcClipToPhone() {
            if (!currentPcClip) {
                showToast("No hay texto para copiar");
                return;
            }
            try {
                await navigator.clipboard.writeText(currentPcClip);
                showToast("✓ ¡Copiado al portapapeles del iPhone!");
            } catch (err) {
                // Fallback for iOS Safari
                const ta = document.createElement("textarea");
                ta.value = currentPcClip;
                document.body.appendChild(ta);
                ta.select();
                document.execCommand("copy");
                document.body.removeChild(ta);
                showToast("✓ ¡Copiado al iPhone!");
            }
        }

        async function pasteFromPhoneClipboard() {
            try {
                const text = await navigator.clipboard.readText();
                if (text) {
                    document.getElementById("phoneTextInput").value = text;
                    showToast("Texto pegado en el campo");
                } else {
                    showToast("Portapapeles de iOS vacío");
                }
            } catch (err) {
                showToast("Presiona dentro del cuadro y toca 'Pegar'");
                document.getElementById("phoneTextInput").focus();
            }
        }

        async function sendTextToPc() {
            const input = document.getElementById("phoneTextInput");
            const text = input.value.trim();
            if (!text) {
                showToast("Escribe o pega algo primero");
                return;
            }

            try {
                const res = await fetch("/api/clipboard", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({ text: text, device: "iPhone Web" })
                });

                if (res.ok) {
                    showToast("✓ ¡Enviado a Linux!");
                    input.value = "";
                    fetchHistory();
                } else {
                    showToast("Error al enviar a PC");
                }
            } catch (err) {
                showToast("Error de conexión con Linux");
            }
        }

        async function handleFileSelection(files) {
            if (!files || files.length === 0) return;
            const statusEl = document.getElementById("uploadStatusText");
            statusEl.style.display = "block";

            for (let i = 0; i < files.length; i++) {
                const file = files[i];
                statusEl.innerText = `Enviando (${i + 1}/${files.length}): ${file.name}...`;
                showToast(`Enviando ${file.name}...`);

                try {
                    const res = await fetch(`/api/upload?name=${encodeURIComponent(file.name)}`, {
                        method: "POST",
                        headers: {
                            "Content-Type": file.type || "application/octet-stream",
                            "X-Filename": file.name
                        },
                        body: file
                    });

                    if (res.ok) {
                        showToast(`✓ ${file.name} guardado en PC`);
                    } else {
                        showToast(`Error al subir ${file.name}`);
                    }
                } catch (e) {
                    showToast("Error de conexión al enviar archivo");
                }
            }

            statusEl.innerText = `✓ ${files.length} archivo(s) transferidos con éxito`;
            setTimeout(() => { statusEl.style.display = "none"; }, 4000);
            document.getElementById("fileUploadInput").value = "";
        }

        function selectHistoryItem(text) {
            currentPcClip = text;
            document.getElementById("pcClipText").innerText = text;
            copyPcClipToPhone();
        }

        function showShortcutGuide() {
            const host = window.location.host;
            alert(
                "Configura tu Atajo de iOS en 30 segundos:\n\n" +
                "1. Abre la app Atajos en iPhone y crea uno nuevo.\n" +
                "2. Agrega la acción 'Obtener portapapeles'.\n" +
                "3. Agrega la acción 'Obtener contenido de URL':\n" +
                "   • URL: http://" + host + "/api/clipboard\n" +
                "   • Método: POST\n" +
                "   • Cuerpo: JSON -> clave 'text' = [Portapapeles]\n" +
                "4. En Ajustes > Accesibilidad > Tocar > Toque Posterior, selecciona este atajo.\n\n" +
                "¡Listo! Al tocar 2 veces detrás de tu iPhone, lo que tengas copiado se irá a tu PC."
            );
        }

        // Live WebSocket Listener for immediate sync
        function setupWebSocket() {
            const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
            const wsUrl = `${proto}//${window.location.host}/ws/clipboard`;
            const ws = new WebSocket(wsUrl);

            ws.onopen = () => {
                document.getElementById("statusBadge").style.background = "rgba(52, 211, 153, 0.12)";
                document.getElementById("statusText").innerText = "En línea";
            };

            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    if (data.type === 'clipboard_updated' && data.item) {
                        updatePcClipDisplay(data.item.text);
                        fetchHistory();
                        if (data.item.source === 'Linux (Wayland)') {
                            showToast("🖥️ Nuevo texto en PC copiado");
                        }
                    }
                } catch (e) {}
            };

            ws.onclose = () => {
                document.getElementById("statusBadge").style.background = "rgba(239, 68, 68, 0.12)";
                document.getElementById("statusText").innerText = "Reconectando...";
                setTimeout(setupWebSocket, 3000);
            };
        }

        // Initialize
        fetchClipboard();
        fetchHistory();
        setupWebSocket();
    </script>
</body>
</html>
"####;
