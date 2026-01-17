use anyhow::{anyhow, Result};
use image::{ImageFormat, RgbImage};
use v4l::format::FourCC;
use v4l::io::mmap::Stream;
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;
use v4l::Device;

/// Configuración para inicializar la captura de vídeo.
pub struct CaptureConfig {
    pub camera_path: String,
    pub fourcc: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

/// Adaptador para la captura física de frames usando V4L2.
pub struct V4l2Capture {
    stream: Stream<'static>,
    fourcc: FourCC,
    width: u32,
    height: u32,
}

impl V4l2Capture {
    /// Abre el dispositivo de cámara y configura el formato y el flujo de memoria mapeada (MMAP).
    pub fn open(cfg: &CaptureConfig) -> Result<Self> {
        let dev = Device::with_path(&cfg.camera_path)?;
        
        // 1. Configurar Formato
        let mut fmt = dev.format()?;
        let b = cfg.fourcc.as_bytes();
        if b.len() != 4 {
            return Err(anyhow!("FourCC debe tener 4 caracteres"));
        }
        fmt.fourcc = v4l::FourCC::new(&[b[0], b[1], b[2], b[3]]);
        fmt.width = cfg.width;
        fmt.height = cfg.height;
        
        // Aplicar formato (el driver puede ajustar los valores a los más cercanos soportados)
        let actual_fmt = dev.set_format(&fmt)?;
        
        // 2. Configurar FPS (Frame Interval)
        let mut params = dev.params()?;
        params.interval.numerator = 1;
        params.interval.denominator = cfg.fps;
        let _ = dev.set_params(&params);

        // 3. Inicializar Stream (MMAP)
        // Usamos Box::leak para que el dispositivo viva tanto como el stream 'static
        let dev_static: &'static Device = Box::leak(Box::new(dev));
        let stream = Stream::with_buffers(dev_static, v4l::buffer::Type::VideoCapture, 4)?;

        tracing::info!(
            "Cámara abierta: {}x{} [{}] a {} FPS", 
            actual_fmt.width, actual_fmt.height, actual_fmt.fourcc, cfg.fps
        );

        Ok(Self {
            stream,
            fourcc: actual_fmt.fourcc,
            width: actual_fmt.width,
            height: actual_fmt.height,
        })
    }

    /// Captura el siguiente frame y lo devuelve en formato RGB (para inferencia) y JPEG (para web).
    pub fn next_rgb_and_jpeg(&mut self) -> Result<(RgbImage, Vec<u8>, u32, u32)> {
        let (data, _) = self.stream.next()?;
        let fcc_str = self.fourcc.str().map_err(|_| anyhow!("FourCC inválido"))?;

        match fcc_str {
            "MJPG" => {
                // MJPG es básicamente una secuencia de JPEGs
                let img = image::load_from_memory_with_format(data, ImageFormat::Jpeg)?;
                let rgb = img.to_rgb8();
                Ok((rgb, data.to_vec(), self.width, self.height))
            }
            "YUYV" => {
                // Conversión manual de YUYV a RGB
                let rgb = yuyv_to_rgb(data, self.width, self.height);
                
                // Comprimir a JPEG para el frontend
                let mut jpeg = Vec::new();
                let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 80);
                enc.encode(rgb.as_raw(), self.width, self.height, image::ExtendedColorType::Rgb8)?;
                
                Ok((rgb, jpeg, self.width, self.height))
            }
            _ => Err(anyhow!("Formato de cámara {} no soportado por este pipeline", fcc_str)),
        }
    }
}

/// Convierte un buffer YUYV (YUV 4:2:2) a una RgbImage de forma eficiente.
fn yuyv_to_rgb(yuyv: &[u8], w: u32, h: u32) -> RgbImage {
    let mut out = RgbImage::new(w, h);
    
    // Cada bloque de 4 bytes en YUYV define 2 píxeles: [Y0, U, Y1, V]
    // Píxel 1: (Y0, U, V) | Píxel 2: (Y1, U, V)
    for (i, chunk) in yuyv.chunks_exact(4).enumerate() {
        let y0 = chunk[0] as f32;
        let u  = chunk[1] as f32 - 128.0;
        let y1 = chunk[2] as f32;
        let v  = chunk[3] as f32 - 128.0;

        // Fórmulas de conversión estándar BT.601
        let r0 = (y0 + 1.402 * v).clamp(0.0, 255.0) as u8;
        let g0 = (y0 - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
        let b0 = (y0 + 1.772 * u).clamp(0.0, 255.0) as u8;
        
        let r1 = (y1 + 1.402 * v).clamp(0.0, 255.0) as u8;
        let g1 = (y1 - 0.344136 * u - 0.714136 * v).clamp(0.0, 255.0) as u8;
        let b1 = (y1 + 1.772 * u).clamp(0.0, 255.0) as u8;

        let pixel_idx = i as u32 * 2;
        let x = pixel_idx % w;
        let y = pixel_idx / w;
        
        if y < h {
            out.put_pixel(x, y, image::Rgb([r0, g0, b0]));
            if x + 1 < w {
                out.put_pixel(x + 1, y, image::Rgb([r1, g1, b1]));
            }
        }
    }
    out
}
