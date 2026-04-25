//! Tipos base utilizados en la evaluación de distancias y similitudes.
//!
//! Define los tipos fundamentales como `Distance` y `Similarity`.

use std::fmt;
use std::ops::{Add, Sub};

/// Newtype para distancia de edición, garantizando valores no negativos.
///
/// Implementa `PartialOrd` y `PartialEq` para comparaciones en el BK-Tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Distance(u32);

impl Distance {
    /// Crea una nueva distancia. Panica si el valor es negativo (imposible por diseño).
    ///
    /// # Panics
    ///
    /// Panics si se intenta crear con valor negativo (previene overflow en cálculos).
    #[inline]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    /// Convierte a `f64` para cálculos de similitud.
    #[inline]
    pub fn as_f64(self) -> f64 {
        self.0 as f64
    }

    /// Obtiene el valor crudo para operaciones internas.
    #[inline]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl Add for Distance {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Distance {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl From<u32> for Distance {
    #[inline]
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Newtype para similitud normalizada en [0.0, 1.0].
///
/// Garantiza invariantes en tiempo de construcción para evitar errores en búsquedas.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Similarity(f64);

impl Similarity {
    /// Umbral por defecto para búsquedas difusas (85% similitud).
    pub const DEFAULT_THRESHOLD: f64 = 0.85;

    /// Crea una nueva similitud validando el rango.
    ///
    /// # Errors
    ///
    /// Retorna error si `value` está fuera de `[0.0, 1.0]`.
    pub fn new(value: f64) -> Result<Self, crate::error::FuzzySearchError> {
        if (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(crate::error::FuzzySearchError::InvalidSimilarity { value })
        }
    }

    /// Constructor seguro desde umbral, usado en búsquedas BK-Tree.
    #[inline]
    pub fn from_threshold(threshold: f64) -> Self {
        Self(threshold.clamp(0.0, 1.0))
    }

    /// Convierte distancia a similitud usando longitud máxima como normalizador.
    ///
    /// ```math
    /// similarity = 1.0 - \frac{distance}{\max(len_a, len_b)}
    /// ```
    #[inline]
    pub fn from_distance(distance: Distance, max_len: usize) -> Self {
        if max_len == 0 {
            Self(1.0)
        } else {
            Self((1.0 - distance.as_f64() / max_len as f64).clamp(0.0, 1.0))
        }
    }

    /// Verifica si cumple el umbral mínimo para inclusión en resultados.
    #[inline]
    pub fn meets_threshold(self, threshold: Self) -> bool {
        self.0 >= threshold.0
    }

    /// Obtiene valor crudo para cálculos internos.
    #[inline]
    pub fn raw(self) -> f64 {
        self.0
    }
}

impl fmt::Display for Similarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}%", self.0 * 100.0)
    }
}