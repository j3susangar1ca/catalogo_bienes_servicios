use thiserror::Error;

/// Errores personalizados para el motor de búsqueda difusa.
///
/// Implementa `std::error::Error` para integración con ecosistema Rust.
#[derive(Error, Debug)]
pub enum FuzzySearchError {
    /// Error al normalizar texto Unicode.
    #[error("Unicode normalization failed: {0}")]
    UnicodeNormalization(String),

    /// Similitud fuera de rango válido [0.0, 1.0].
    #[error("Invalid similarity value: {value}. Must be in [0.0, 1.0]")]
    InvalidSimilarity {
        /// Valor inválido proporcionado.
        value: f64,
    },

    /// Distancia negativa (imposible por diseño, indica bug lógico).
    #[error("Negative distance computed: {value}. This indicates a logic error.")]
    NegativeDistance {
        /// Valor negativo computado.
        value: i64,
    },

    /// Árbol BK vacío en operación que requiere elementos.
    #[error("BK-Tree is empty; cannot perform search without indexed data")]
    EmptyTree,

    /// Error de concurrencia en operaciones paralelas con Rayon.
    #[error("Parallel execution failed: {0}")]
    ParallelExecution(String),

    /// Límite de memoria excedido en buffer de DP matrix.
    #[error("Memory limit exceeded for dynamic programming buffer")]
    MemoryLimitExceeded,
}

/// Alias para `Result<T, FuzzySearchError>`.
pub type Result<T> = std::result::Result<T, FuzzySearchError>;