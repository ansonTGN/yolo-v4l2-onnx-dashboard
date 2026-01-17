# YOLO Camera Dashboard (Rust)

**Languages / Idiomes:**

* [Español](#español)
* [English](#english)
* [Català](#català)

---

## Español

### Visión general

**YOLO Camera Dashboard** es una aplicación en Rust que combina:

* Captura de vídeo desde cámaras **V4L2** (Linux)
* Inferencia **YOLO en ONNX Runtime** (con soporte CUDA si está disponible)
* Un dashboard web servido por **Axum** con streaming por **WebSocket**
* (Opcional) **Narración por voz**: un VLM en **Ollama (Moondream)** describe la escena y **Piper TTS** la lee por altavoz

El servidor se levanta por defecto en `http://0.0.0.0:8090` y sirve el frontend desde `./static`. 

---

### Qué puedes hacer con este proyecto

| Capacidades                    | Detalles                                                                                            |
| ------------------------------ | --------------------------------------------------------------------------------------------------- |
| Detección en tiempo real       | Inferencia YOLO sobre el frame actual, con umbrales configurables (conf / IoU / tamaño de entrada)  |
| Dashboard web                  | Control de cámara + selección de modelo ONNX + overlay de detecciones + métricas (ms/FPS)           |
| Pipeline “hot reload”          | Cambia cámara / formato / modelo y el worker recarga recursos automáticamente                       |
| Narración opcional (VLM + TTS) | Moondream (Ollama) genera una frase corta y Piper la reproduce con `aplay`                          |

---

### Arquitectura

El proyecto sigue un enfoque **hexagonal**: `domain` (modelos y reglas), `application` (casos de uso/puertos), `adapters` (infraestructura: V4L2/ONNX/HTTP).

```mermaid
flowchart LR
  UI[Web UI (static/)] <-- WebSocket + REST --> HTTP[Axum HTTP Adapter]
  HTTP --> APP[Application Services]
  APP -->|ports| DOM[Domain]
  APP --> V4L2[V4L2 Adapter]
  APP --> ONNX[ONNX Adapter]
  ONNX --> ORT[ONNX Runtime]
  APP --> PIPE[Pipeline Worker]
  PIPE --> SPEECH[SpeechService (opcional)]
  SPEECH --> OLLAMA[Ollama (moondream)]
  SPEECH --> PIPER[Piper TTS + aplay]
```

El **Pipeline Worker** corre en un hilo dedicado: captura frame, ejecuta inferencia, publica metadatos + JPEG por un canal broadcast para los clientes WebSocket, y opcionalmente alimenta el servicio de voz.

---

### Requisitos

**Sistema**

* Linux con **V4L2** (cámaras en `/dev/video*`)
* Rust (stable)
* Recomendado: GPU NVIDIA + drivers si quieres acelerar con CUDA (el proyecto compila con soporte CUDA en ONNX Runtime). 

**Audio (solo si activas narración)**

* `aplay` (paquete `alsa-utils` en muchas distros) 

---

### Instalación y ejecución

```bash
# 1) Compilar
cargo build --release

# 2) Ejecutar
cargo run --release
```

Abre el dashboard en:

```text
http://localhost:8090
```

(El servidor escucha en `0.0.0.0:8090` por defecto). 

---

## Modelos: qué se usa y cómo instalarlos en el host

Este repositorio trabaja con **tres familias** de modelos:

1. **YOLO (detección) en formato ONNX**
2. **Moondream (VLM) en Ollama** (opcional, narración)
3. **Piper TTS** + **modelo de voz** (opcional, narración)

---

### 1) YOLO ONNX (detección)

El catálogo de modelos ONNX busca ficheros `.onnx` (p. ej. en `./models`) y valida su existencia antes de configurar el pipeline.

**Configuración por defecto**: el servidor anuncia/propone un `models/yolo11n.onnx` con parámetros típicos (tamaño de entrada y umbrales). 

#### Opción A (recomendada): exportar a ONNX con Ultralytics

Ultralytics documenta el modo *export* para generar ONNX desde un modelo YOLO. ([Ultralytics Docs][1])

Ejemplo en el host:

```bash
# Crear venv (opcional)
python3 -m venv .venv
source .venv/bin/activate

# Instalar ultralytics
pip install -U ultralytics

# Exportar YOLO11n a ONNX (ajusta el nombre del modelo si usas otro)
yolo export model=yolo11n.pt format=onnx imgsz=640
```

Luego coloca el ONNX en la carpeta esperada por el repo:

```bash
mkdir -p models
mv yolo11n.onnx models/yolo11n.onnx
```

#### Opción B: usar un ONNX ya exportado

Si ya tienes un `.onnx` compatible (exportado desde Ultralytics), guárdalo en `models/` o donde prefieras y selecciónalo desde el dashboard (el backend expone exploración/listado de ONNX).

---

### 2) Moondream en Ollama (VLM, opcional)

El servicio de voz llama a un endpoint local `http://localhost:11434/api/generate` y usa por defecto el modelo `moondream:latest` para producir una frase corta describiendo la imagen.

**Instalar Ollama** (según documentación oficial): ([Ollama Documentation][2])

```bash
# Linux (ejemplo habitual)
curl -fsSL https://ollama.com/install.sh | sh
```

Descargar el modelo:

```bash
ollama pull moondream:latest
```

Verificación rápida:

```bash
ollama list
curl -s http://localhost:11434/api/tags | head
```

---

### 3) Piper TTS + modelos de voz (opcional)

El proyecto invoca un binario local en:

* `./piper_voice/piper/piper`
* y un modelo de voz en:

  * `./piper_voice/en_US-lessac-medium.onnx` (por defecto en el código)

Luego reproduce audio RAW con:

```text
aplay -r 22050 -f S16_LE -t raw
```

Todo esto está implementado en `SpeechService`.

#### Descargar modelos de voz (ejemplo)

Puedes obtener estos modelos desde repositorios públicos (p. ej., Hugging Face). ([Hugging Face][3])

Ejemplo:

```bash
mkdir -p piper_voice

# Modelo en inglés (ejemplo consistente con el código)
wget -O piper_voice/en_US-lessac-medium.onnx \
  "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"

# Modelo en español (si quieres extender el selector/uso en código)
wget -O piper_voice/es_ES-sharvard-medium.onnx \
  "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/sharvard/medium/es_ES-sharvard-medium.onnx"
```

#### Binario Piper

Coloca el ejecutable en `piper_voice/piper/piper` (ajusta permisos):

```bash
chmod +x piper_voice/piper/piper
```

---

### Endpoints principales (backend)

El backend expone una API REST y un WebSocket para el streaming de frames.

* `GET /api/cameras` — lista cámaras
* `GET /api/formats?camera_path=...` — formatos disponibles
* `GET /api/controls?camera_path=...` — controles (exposición, ganancia, etc.)
* `POST /api/controls` — aplicar controles
* `GET /api/files?path=...` — explorador (útil para localizar `.onnx`)
* `POST /api/pipeline` — configura cámara + modo + modelo
* `GET /ws/stream` — WebSocket (metadatos + JPEG)

---

### Troubleshooting (rápido y práctico)

* **No veo cámaras**: verifica permisos sobre `/dev/video*` (grupos `video`/udev).
* **El modelo no carga**: confirma que el `.onnx` existe y que la ruta coincide; el servicio valida el modelo antes de arrancar el hardware.
* **No hay narración**:

  * Ollama debe estar levantado y con `moondream:latest` disponible.
  * Debe existir `piper_voice/en_US-lessac-medium.onnx` y `piper_voice/piper/piper`. 
  * `aplay` debe estar instalado. 

---

## English

### Overview

**YOLO Camera Dashboard** is a Rust application that combines:

* **V4L2** camera capture (Linux)
* **YOLO inference via ONNX Runtime** (CUDA-capable if available)
* **Axum** backend serving a web dashboard + **WebSocket** streaming
* (Optional) **Voice narration**: a VLM in **Ollama (Moondream)** describes the scene and **Piper TTS** speaks it

The server runs on `http://0.0.0.0:8090` by default and serves the frontend from `./static`. 

---

### Key features

| Feature                        | Notes                                                               |
| ------------------------------ | ------------------------------------------------------------------- |
| Real-time detection            | YOLO ONNX inference with configurable thresholds and input size     |
| Web dashboard                  | Camera controls + ONNX model selection + detection overlay + ms/FPS |
| Hot reload pipeline            | Switch camera/format/model and the worker reloads automatically     |
| Optional narration (VLM + TTS) | Moondream generates one short sentence, Piper plays it via `aplay`  |

---

### Architecture (Hexagonal)

The repo is structured around `domain` (models), `application` (use-cases/ports), and `adapters` (infra: V4L2/ONNX/HTTP).

```mermaid
flowchart LR
  UI[Web UI (static/)] <-- WebSocket + REST --> HTTP[Axum HTTP Adapter]
  HTTP --> APP[Application Services]
  APP -->|ports| DOM[Domain]
  APP --> V4L2[V4L2 Adapter]
  APP --> ONNX[ONNX Adapter]
  ONNX --> ORT[ONNX Runtime]
  APP --> PIPE[Pipeline Worker]
  PIPE --> SPEECH[SpeechService (optional)]
  SPEECH --> OLLAMA[Ollama (moondream)]
  SPEECH --> PIPER[Piper TTS + aplay]
```

The **Pipeline Worker** runs in its own thread: capture → infer → broadcast meta+JPEG to WebSocket clients; optionally feed the voice service.

---

### Requirements

* Linux + V4L2 cameras (`/dev/video*`)
* Rust (stable)
* Optional: NVIDIA drivers for CUDA acceleration. 
* Optional audio: `aplay` (alsa-utils). 

---

### Build & run

```bash
cargo build --release
cargo run --release
```

Open:

```text
http://localhost:8090
```

Default bind: `0.0.0.0:8090`. 

---

## Models: what’s used & how to install them on the host

### 1) YOLO ONNX

The model catalog expects `.onnx` files (commonly under `./models`) and validates them before configuring the pipeline.
Default config suggests `models/yolo11n.onnx`. 

**Recommended: export with Ultralytics**. ([Ultralytics Docs][1])

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -U ultralytics
yolo export model=yolo11n.pt format=onnx imgsz=640

mkdir -p models
mv yolo11n.onnx models/yolo11n.onnx
```

---

### 2) Moondream via Ollama (optional)

The voice service calls `http://localhost:11434/api/generate` using `moondream:latest`.
Install Ollama (official): ([Ollama Documentation][2])

```bash
curl -fsSL https://ollama.com/install.sh | sh
ollama pull moondream:latest
```

---

### 3) Piper TTS + voice model (optional)

The code runs:

* `./piper_voice/piper/piper`
* with voice model `./piper_voice/en_US-lessac-medium.onnx`
* then plays raw audio via `aplay`. 

Example voice downloads (Hugging Face): ([Hugging Face][3])

```bash
mkdir -p piper_voice

wget -O piper_voice/en_US-lessac-medium.onnx \
  "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"

wget -O piper_voice/es_ES-sharvard-medium.onnx \
  "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/sharvard/medium/es_ES-sharvard-medium.onnx"

chmod +x piper_voice/piper/piper
```

---

### Backend endpoints (main)

REST + WebSocket streaming are exposed.

* `GET /api/cameras`
* `GET /api/formats?camera_path=...`
* `GET /api/controls?camera_path=...`
* `POST /api/controls`
* `GET /api/files?path=...`
* `POST /api/pipeline`
* `GET /ws/stream`

---

## Català

### Visió general

**YOLO Camera Dashboard** és una aplicació en Rust que combina:

* Captura de vídeo amb **V4L2** (Linux)
* Inferència **YOLO amb ONNX Runtime** (amb suport CUDA si n’hi ha)
* Backend **Axum** amb dashboard web i streaming per **WebSocket**
* (Opcional) **Narració**: un VLM a **Ollama (Moondream)** descriu l’escena i **Piper TTS** la llegeix

Servidor per defecte a `http://0.0.0.0:8090`, i frontend servit des de `./static`. 

---

### Funcionalitats

| Funcionalitat           | Detall                                                     |
| ----------------------- | ---------------------------------------------------------- |
| Detecció en temps real  | YOLO ONNX amb paràmetres configurables                     |
| Dashboard web           | Controls de càmera, selecció de model, overlay i mètriques |
| Hot reload del pipeline | En canviar càmera/model es recarreguen recursos            |
| Narració opcional       | Moondream genera una frase curta i Piper la reprodueix     |

---

### Arquitectura (hexagonal)

Estructura: `domain` (models), `application` (casos d’ús/ports), `adapters` (infra: V4L2/ONNX/HTTP).

```mermaid
flowchart LR
  UI[Web UI (static/)] <-- WebSocket + REST --> HTTP[Axum HTTP Adapter]
  HTTP --> APP[Application Services]
  APP -->|ports| DOM[Domain]
  APP --> V4L2[V4L2 Adapter]
  APP --> ONNX[ONNX Adapter]
  ONNX --> ORT[ONNX Runtime]
  APP --> PIPE[Pipeline Worker]
  PIPE --> SPEECH[SpeechService (opcional)]
  SPEECH --> OLLAMA[Ollama (moondream)]
  SPEECH --> PIPER[Piper TTS + aplay]
```

---

### Execució

```bash
cargo build --release
cargo run --release
```

Obre:

```text
http://localhost:8090
```

Bind per defecte: `0.0.0.0:8090`. 

---

## Models: què s’usa i com instal·lar-ho al host

### 1) YOLO en ONNX

Es requereixen fitxers `.onnx` (habitualment a `./models`) i es valida el model abans de configurar el pipeline.
Configuració per defecte: `models/yolo11n.onnx`. 

**Recomanat: exportar amb Ultralytics**. ([Ultralytics Docs][1])

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -U ultralytics
yolo export model=yolo11n.pt format=onnx imgsz=640

mkdir -p models
mv yolo11n.onnx models/yolo11n.onnx
```

---

### 2) Moondream a Ollama (opcional)

El servei de veu crida `http://localhost:11434/api/generate` i usa `moondream:latest`.
Instal·lació d’Ollama (oficial): ([Ollama Documentation][2])

```bash
curl -fsSL https://ollama.com/install.sh | sh
ollama pull moondream:latest
```

---

### 3) Piper TTS + veu (opcional)

Executa `./piper_voice/piper/piper` amb el model `./piper_voice/en_US-lessac-medium.onnx` i reprodueix via `aplay`. 
Exemples de descàrrega (Hugging Face): ([Hugging Face][3])

```bash
mkdir -p piper_voice

wget -O piper_voice/en_US-lessac-medium.onnx \
  "https://huggingface.co/rhasspy/piper-voices/resolve/main/en/en_US/lessac/medium/en_US-lessac-medium.onnx"

wget -O piper_voice/es_ES-sharvard-medium.onnx \
  "https://huggingface.co/rhasspy/piper-voices/resolve/main/es/es_ES/sharvard/medium/es_ES-sharvard-medium.onnx"

chmod +x piper_voice/piper/piper
```

[1]: https://docs.ultralytics.com/modes/export/?utm_source=chatgpt.com "Model Export with Ultralytics YOLO"
[2]: https://docs.ollama.com/linux?utm_source=chatgpt.com "Linux"
[3]: https://huggingface.co/rhasspy/piper-voices/tree/main/en/en_US/lessac/medium?utm_source=chatgpt.com "rhasspy/piper-voices at main"
