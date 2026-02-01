use anyhow::Result;
use image::{imageops::FilterType, RgbImage};
use ndarray::{s, Array4, ArrayViewD, Axis, IxDyn};
use ort::execution_providers::CUDAExecutionProvider;
use ort::session::Session;
use ort::value::Value;
use std::fs;

use crate::domain::detection::Detection;
use crate::domain::model::YoloParams;

pub struct OnnxYoloEngine {
    session: Session,
}

impl OnnxYoloEngine {
    pub fn load(path: &str) -> Result<Self> {
        let mut builder = Session::builder()?.with_intra_threads(4)?;

        // CUDA es opcional: si está disponible se registra, si no continuamos en CPU.
        let cuda = CUDAExecutionProvider::default().build();
        if let Ok(builder_with_cuda) = builder.clone().with_execution_providers([cuda]) {
            builder = builder_with_cuda;
        }

        // Con `ort` sin default-features, usamos commit_from_memory.
        let model_bytes = fs::read(path)?;
        let session = builder.commit_from_memory(&model_bytes)?;

        Ok(Self { session })
    }

    pub fn infer(&mut self, rgb: &RgbImage, params: &YoloParams) -> Result<Vec<Detection>> {
        let classes = [
            "persona", "bicicleta", "coche", "motocicleta", "avión", "autobús", "tren", "camión", "barco",
            "semáforo", "hidrante", "señal de stop", "parquímetro", "banco", "pájaro", "gato", "perro",
            "caballo", "oveja", "vaca", "elefante", "oso", "cebra", "jirafa", "mochila", "paraguas",
            "bolso", "corbata", "maleta", "frisbee", "esquís", "snowboard", "pelota", "cometa",
            "bate de béisbol", "guante de béisbol", "monopatín", "tabla de surf", "raqueta de tenis",
            "botella", "copa de vino", "taza", "tenedor", "cuchillo", "cuchara", "tazón", "plátano",
            "manzana", "sándwich", "naranja", "brócoli", "zanahoria", "perrito caliente", "pizza",
            "donut", "pastel", "silla", "sofá", "planta", "cama", "mesa", "inodoro", "televisor",
            "portátil", "ratón", "mando", "teclado", "móvil", "microondas", "horno", "tostadora",
            "fregadero", "nevera", "libro", "reloj", "jarrón", "tijeras", "peluche", "secador", "cepillo"
        ];

        let imgsz = params.input_size as usize;
        let resized = image::imageops::resize(rgb, imgsz as u32, imgsz as u32, FilterType::Nearest);

        let mut input = Array4::<f32>::zeros((1, 3, imgsz, imgsz));
        for (x, y, pixel) in resized.enumerate_pixels() {
            input[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0;
            input[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0;
            input[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0;
        }

        let input_shape = vec![1, 3, imgsz as i64, imgsz as i64];
        let input_tensor = Value::from_array((input_shape, input.into_raw_vec()))?;

        let outputs = self.session.run(ort::inputs![input_tensor])?;
        let (shape_out, data_out) = outputs[0].try_extract_tensor::<f32>()?;

        let dims: Vec<usize> = shape_out.into_iter().map(|&x| x as usize).collect();
        let array_view = ArrayViewD::from_shape(IxDyn(&dims), data_out)?;
        let view = array_view.index_axis(Axis(0), 0);

        let num_candidates = view.shape()[1];
        let sx = rgb.width() as f32 / imgsz as f32;
        let sy = rgb.height() as f32 / imgsz as f32;

        let mut detections = Vec::new();

        for i in 0..num_candidates {
            let scores = view.slice(s![4.., i]);
            let (class_id, &max_score) = scores
                .indexed_iter()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .unwrap();

            if max_score > params.conf_threshold {
                let cx = view[[0, i]];
                let cy = view[[1, i]];
                let w = view[[2, i]];
                let h = view[[3, i]];

                detections.push(Detection {
                    x1: (cx - w / 2.0) * sx,
                    y1: (cy - h / 2.0) * sy,
                    x2: (cx + w / 2.0) * sx,
                    y2: (cy + h / 2.0) * sy,
                    score: max_score,
                    class_id,
                    label: classes.get(class_id).unwrap_or(&"objeto").to_string(),
                });
            }
        }

        detections.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        Ok(detections.into_iter().take(params.max_detections).collect())
    }
}
