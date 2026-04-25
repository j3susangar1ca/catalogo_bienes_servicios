// ============================================================
//  MODULE: embedding_bridge
//  Gestión de tensores de entrada: normalización, padding,
//  y (mock) integración con ONNX Runtime.
//
//  En producción, reemplazar `mock_encode` por una llamada real
//  a `ort::Session::run(...)` con el modelo BERT/ONNX cargado.
// ============================================================

use crate::vector_math::{normalize_inplace, DIM};

// ─────────────────────────────────────────────────────────────
//  TENSOR DE EMBEDDING
// ─────────────────────────────────────────────────────────────
/// Wrapper de un embedding con metadatos.
#[derive(Clone)]
pub struct Embedding {
    /// Valores del vector (f32, longitud = DIM).
    pub data: Vec<f32>,
    /// Texto original del que se generó.
    pub source_text: String,
    /// ¿Ya normalizado (‖v‖=1)?
    pub is_normalized: bool,
}

impl Embedding {
    /// Crea un embedding desde un slice de f32.
    pub fn from_raw(data: Vec<f32>, source_text: impl Into<String>) -> Self {
        assert_eq!(data.len(), DIM, "Embedding: se esperan {} dimensiones", DIM);
        Self {
            data,
            source_text: source_text.into(),
            is_normalized: false,
        }
    }

    /// Normaliza el embedding in-place.
    pub fn normalize(&mut self) {
        normalize_inplace(&mut self.data);
        self.is_normalized = true;
    }

    /// Retorna slice de los datos — sin copia.
    #[inline(always)]
    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }
}

// ─────────────────────────────────────────────────────────────
//  BRIDGE — Interfaz de codificación de texto
// ─────────────────────────────────────────────────────────────
/// Trait que abstrae cualquier backend de embeddings.
/// Implementa con ONNX Runtime en producción.
pub trait EmbeddingBackend: Send + Sync {
    fn encode(&self, text: &str) -> Embedding;
}

// ─────────────────────────────────────────────────────────────
//  MOCK BACKEND — Simulación determinista para pruebas
// ─────────────────────────────────────────────────────────────
/// Genera embeddings sintéticos pero *deterministas*:
/// usa hashing del texto para producir el mismo vector siempre.
/// Esto simula el comportamiento de un modelo real sin necesitar
/// el modelo ONNX cargado.
pub struct MockEmbeddingBackend;

impl EmbeddingBackend for MockEmbeddingBackend {
    fn encode(&self, text: &str) -> Embedding {
        let mut data = vec![0.0f32; DIM];
        mock_encode_text(text, &mut data);
        let mut emb = Embedding::from_raw(data, text);
        emb.normalize();
        emb
    }
}

/// Genera un vector pseudoaleatorio determinista basado en el texto.
/// Algoritmo: hash FNV-1a → semilla → LCG para cada dimensión.
fn mock_encode_text(text: &str, out: &mut [f32]) {
    // FNV-1a hash del texto como semilla.
    let mut hash: u64 = 14695981039346656037;
    for byte in text.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }

    // LCG para llenar el vector.
    let mut state = hash;
    for v in out.iter_mut() {
        state = state.wrapping_mul(6364136223846793005)
                     .wrapping_add(1442695040888963407);
        // Normaliza a [-1, 1]
        *v = ((state >> 33) as f32 / (u32::MAX as f32)) * 2.0 - 1.0;
    }
}

// ─────────────────────────────────────────────────────────────
//  ONNX RUNTIME BRIDGE (stub de producción)
// ─────────────────────────────────────────────────────────────
/// En producción, habilita el feature "onnx" y descomenta:
///
/// ```rust
/// use ort::{Environment, SessionBuilder, Value};
///
/// pub struct OnnxEmbeddingBackend {
///     session: ort::Session,
///     tokenizer: tokenizers::Tokenizer,
/// }
///
/// impl OnnxEmbeddingBackend {
///     pub fn new(model_path: &str, tokenizer_path: &str) -> anyhow::Result<Self> {
///         let env     = Environment::builder().build()?;
///         let session = SessionBuilder::new(&env)?
///             .with_model_from_file(model_path)?;
///         let tokenizer = tokenizers::Tokenizer::from_file(tokenizer_path)?;
///         Ok(Self { session, tokenizer })
///     }
/// }
///
/// impl EmbeddingBackend for OnnxEmbeddingBackend {
///     fn encode(&self, text: &str) -> Embedding {
///         let encoding = self.tokenizer.encode(text, true).unwrap();
///         let ids  : Vec<i64> = encoding.get_ids().iter().map(|&x| x as i64).collect();
///         let mask : Vec<i64> = encoding.get_attention_mask().iter().map(|&x| x as i64).collect();
///         let seq_len = ids.len();
///
///         let input_ids   = Value::from_array(([1, seq_len], ids.as_slice())).unwrap();
///         let attn_mask   = Value::from_array(([1, seq_len], mask.as_slice())).unwrap();
///
///         let outputs = self.session.run(ort::inputs![input_ids, attn_mask]).unwrap();
///
///         // CLS token embedding: outputs[0][0][0][0..DIM]
///         let output_tensor: ort::Tensor<f32> = outputs[0].try_extract().unwrap();
///         let data = output_tensor.view().as_slice().unwrap()[..DIM].to_vec();
///
///         let mut emb = Embedding::from_raw(data, text);
///         emb.normalize();
///         emb
///     }
/// }
/// ```
pub struct OnnxEmbeddingBackend; // placeholder — sin dependencia real en este demo

impl EmbeddingBackend for OnnxEmbeddingBackend {
    fn encode(&self, text: &str) -> Embedding {
        // En demo, delega al mock.
        MockEmbeddingBackend.encode(text)
    }
}

// ─────────────────────────────────────────────────────────────
//  PIPELINE DE CATÁLOGO
// ─────────────────────────────────────────────────────────────
/// Convierte un slice de descripciones de texto en embeddings listos
/// para insertar en el catálogo.
pub fn encode_catalog<B: EmbeddingBackend>(
    backend: &B,
    descriptions: &[&str],
) -> Vec<Embedding> {
    descriptions.iter().map(|desc| backend.encode(desc)).collect()
}
