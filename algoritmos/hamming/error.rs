//! Módulo de errores tipados para el motor Hamming.

use core::fmt;

/// Error tipado para operaciones de distancia de Hamming.
/// 
/// Garantiza que el sistema nunca realice *panic* por longitudes incompatibles;
/// en su lugar, propaga este error a través del `Result` tipado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HammingError {
    /// Las secuencias comparadas no poseen la misma cardinalidad.
    IncompatibleLength { expected: usize, found: usize },
}

impl fmt::Display for HammingError {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HammingError::IncompatibleLength { expected, found } => {
                write!(
                    f,
                    "IncompatibleLength: expected {} elements, found {}",
                    expected, found
                )
            }
        }
    }
}

impl std::error::Error for HammingError {}

/// Alias de resultado especializado para el dominio Hamming.
pub type Result<T> = core::result::Result<T, HammingError>;
