/**
 * YOLO Camera Dashboard - Frontend logic
 */

const el = (id) => document.getElementById(id);
let ws = null;
let lastMeta = null;
let currentBrowserPath = ".";

// --- UTILIDADES ---

function setStatus(text, ok = true) {
    const statusEl = el("status");
    statusEl.textContent = text;
    statusEl.style.color = ok ? "#9fb0d0" : "#ff6b6b";
    if (!ok) console.error("Status Error:", text);
}

async function apiGet(path) {
    const r = await fetch(path);
    if (!r.ok) throw new Error(`HTTP ${r.status} en ${path}`);
    return r.json();
}

async function apiPost(path, body) {
    const r = await fetch(path, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
    });
    const data = await r.json();
    if (!r.ok) throw new Error(data.error || `Error ${r.status}`);
    return data;
}

// --- EXPLORADOR DE ARCHIVOS ---

async function browseFiles(path) {
    try {
        const data = await apiGet(`/api/files?path=${encodeURIComponent(path)}`);
        currentBrowserPath = data.current_path;
        el("currentDir").textContent = `📁 ${currentBrowserPath}`;
        
        const list = el("fileEntries");
        list.innerHTML = "";

        // Botón para subir un nivel
        const up = document.createElement("div");
        up.innerHTML = "🔙 . . / (Subir)";
        up.className = "file-entry-up"; // Puedes añadir estilos en CSS
        up.style.cssText = "cursor:pointer; color:var(--accent); padding:8px; border-bottom:1px solid var(--line);";
        up.onclick = () => browseFiles(currentBrowserPath + "/..");
        list.appendChild(up);

        if (!data.entries || data.entries.length === 0) {
            list.innerHTML += `<div style="padding:10px; color:var(--muted)">Carpeta vacía o sin archivos .onnx</div>`;
        }

        data.entries.forEach(e => {
            const item = document.createElement("div");
            item.textContent = e.is_dir ? `📁 ${e.name}` : `📄 ${e.name}`;
            item.style.cssText = "cursor:pointer; padding:6px; border-bottom: 1px solid rgba(255,255,255,0.05); transition: background 0.2s;";
            item.onmouseover = () => item.style.background = "rgba(110,168,254,0.1)";
            item.onmouseout = () => item.style.background = "transparent";
            
            item.onclick = () => {
                if (e.is_dir) {
                    browseFiles(e.path);
                } else {
                    el("modelPath").value = e.path;
                    el("fileBrowser").style.display = "none";
                }
            };
            list.appendChild(item);
        });
    } catch (e) {
        console.error("Error explorando archivos:", e);
        setStatus("Error al leer archivos del servidor", false);
    }
}

// --- GESTIÓN DE CÁMARA Y MODOS ---

async function loadCameras() {
    try {
        const cams = await apiGet("/api/cameras");
        const sel = el("cameraSelect");
        if (!cams || cams.length === 0) {
            sel.innerHTML = `<option value="">No se detectaron cámaras</option>`;
            return false;
        }
        sel.innerHTML = cams.map(c => `<option value="${c.index}">${c.card} (${c.path})</option>`).join("");
        return true;
    } catch (e) {
        console.error("Error cargando cámaras:", e);
        return false;
    }
}

async function loadModes(idx) {
    try {
        const data = await apiGet(`/api/cameras/${idx}/modes`);
        
        // Llenar Formatos
        el("fourccSelect").innerHTML = data.formats.map(f => 
            `<option value="${f.fourcc}">${f.fourcc} - ${f.description}</option>`
        ).join("");
        
        // Llenar Tamaños
        el("sizeSelect").innerHTML = data.frame_sizes.map(s => 
            `<option value="${s.width}x${s.height}">${s.width}x${s.height}</option>`
        ).join("");
        
        // Llenar FPS
        el("fpsSelect").innerHTML = data.fps_options.map(f => 
            `<option value="${f}">${f} FPS</option>`
        ).join("");

        // Cargar controles hardware de esta cámara
        await loadControls(idx);
    } catch (e) {
        console.error("Error cargando modos:", e);
    }
}

async function loadControls(idx) {
    try {
        const ctrls = await apiGet(`/api/cameras/${idx}/controls`);
        const container = el("controls");
        container.innerHTML = "";
        
        if (!ctrls || ctrls.length === 0) {
            container.innerHTML = `<div class="hint">No hay controles ajustables para esta cámara.</div>`;
            return;
        }

        ctrls.forEach(c => {
            const div = document.createElement("div");
            div.className = "field";
            div.innerHTML = `
                <div style="display:flex; justify-content:space-between">
                    <span>${c.name}</span>
                    <output style="color:var(--accent); font-family:var(--mono); font-size:11px">${c.current_value}</output>
                </div>
                <input type="range" min="${c.minimum}" max="${c.maximum}" step="${c.step}" value="${c.current_value}" 
                oninput="this.previousElementSibling.querySelector('output').value = this.value" 
                onchange="updateControl(${idx}, ${c.id}, this.value)">
            `;
            container.appendChild(div);
        });
    } catch (e) {
        console.error("Error cargando controles:", e);
    }
}

async function updateControl(camIdx, ctrlId, val) {
    try {
        await apiPost(`/api/cameras/${camIdx}/controls`, { values: [[parseInt(ctrlId), parseInt(val)]] });
    } catch (e) {
        setStatus("Error actualizando control", false);
    }
}

// --- PIPELINE Y STREAMING ---

async function apply() {
    try {
        setStatus("Aplicando configuración...");
        const [w, h] = el("sizeSelect").value.split("x").map(Number);
        
        const payload = {
            camera_index: Number(el("cameraSelect").value),
            fourcc: el("fourccSelect").value,
            width: w,
            height: h,
            fps: parseInt(el("fpsSelect").value),
            model_path: el("modelPath").value,
            imgsz: Number(el("imgsz").value),
            conf_thres: parseFloat(el("conf").value),
            iou_thres: parseFloat(el("iou").value),
            max_det: parseInt(el("maxDet").value)
        };
        
        await apiPost("/api/config", payload);
        setStatus("Configuración aplicada con éxito");
    } catch (e) {
        setStatus(`Error al aplicar: ${e.message}`, false);
    }
}

function connectWS() {
    if (ws) ws.close();
    
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    ws = new WebSocket(`${proto}//${location.host}/ws/stream`);
    ws.binaryType = "arraybuffer";

    ws.onopen = () => setStatus("Stream conectado");
    ws.onclose = () => {
        setStatus("Stream desconectado. Reintentando...", false);
        setTimeout(connectWS, 3000);
    };

    ws.onmessage = async (ev) => {
        // Los mensajes JSON contienen metadatos de detección
        if (typeof ev.data === "string") {
            try {
                const msg = JSON.parse(ev.data);
                if (msg.type === "frame") lastMeta = msg.meta;
            } catch (e) {}
            return;
        }

        // Los mensajes binarios son el frame JPEG
        if (!lastMeta) return;

        try {
            const blob = new Blob([ev.data], { type: "image/jpeg" });
            const bitmap = await createImageBitmap(blob);
            const canvas = el("canvas");
            const ctx = canvas.getContext("2d");

            // Ajustar tamaño del canvas si cambió la resolución
            if (canvas.width !== lastMeta.width || canvas.height !== lastMeta.height) {
                canvas.width = lastMeta.width;
                canvas.height = lastMeta.height;
            }

            ctx.drawImage(bitmap, 0, 0);
            
            // Dibujar Detecciones
            ctx.strokeStyle = "#00ff00";
            ctx.lineWidth = 3;
            ctx.fillStyle = "#00ff00";
            ctx.font = "bold 16px monospace";

            (lastMeta.detections || []).forEach(d => {
                const bw = d.x2 - d.x1;
                const bh = d.y2 - d.y1;
                ctx.strokeRect(d.x1, d.y1, bw, bh);
                
                const label = `${d.label} ${(d.score * 100).toFixed(0)}%`;
                const txtW = ctx.measureText(label).width;
                ctx.fillRect(d.x1, d.y1 - 22, txtW + 10, 22);
                ctx.fillStyle = "#000";
                ctx.fillText(label, d.x1 + 5, d.y1 - 6);
                ctx.fillStyle = "#00ff00";
            });

            // Liberar memoria del bitmap
            bitmap.close();

            // Actualizar métricas en el UI
            el("metricFps").textContent = `FPS: ${lastMeta.fps_est.toFixed(1)}`;
            el("metricInfer").textContent = `Infer: ${lastMeta.infer_ms.toFixed(1)}ms`;
        } catch (e) {
            console.error("Error renderizando frame:", e);
        }
    };
}

// --- INICIALIZACIÓN ---

async function init() {
    try {
        console.log("Inicializando Dashboard...");
        
        // 1. Cargar configuración inicial del servidor
        const config = await apiGet("/api/config").catch(() => null);
        if (config) {
            el("modelPath").value = config.model_path;
            el("imgsz").value = config.imgsz;
            el("maxDet").value = config.max_det;
            el("conf").value = config.conf_thres;
            el("iou").value = config.iou_thres;
        }

        // 2. Cargar lista de cámaras
        const hasCameras = await loadCameras();
        
        if (hasCameras) {
            const initialCam = el("cameraSelect").value;
            if (initialCam !== "") {
                await loadModes(initialCam);
            }
        } else {
            setStatus("No se detectaron cámaras en /dev/video*", false);
        }

        // 3. Configurar Event Listeners
        el("cameraSelect").onchange = (e) => {
            if (e.target.value !== "") loadModes(e.target.value);
        };
        
        el("applyCameraMode").onclick = apply;
        el("applyModel").onclick = apply;
        
        el("refreshControls").onclick = () => {
            const idx = el("cameraSelect").value;
            if (idx !== "") loadControls(idx);
        };

        el("browseModel").onclick = (e) => {
            e.preventDefault();
            const browser = el("fileBrowser");
            if (browser.style.display === "none") {
                browser.style.display = "block";
                browseFiles(currentBrowserPath);
            } else {
                browser.style.display = "none";
            }
        };

        // 4. Iniciar WebSocket
        connectWS();
        
        console.log("Dashboard listo.");
    } catch (err) {
        console.error("Error crítico en init:", err);
        setStatus(`Error de inicialización: ${err.message}`, false);
    }
}

// Arrancar cuando el DOM esté listo
document.addEventListener("DOMContentLoaded", init);