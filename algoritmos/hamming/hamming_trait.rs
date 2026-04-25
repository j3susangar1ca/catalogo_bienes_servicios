//! Trait `HammingTarget` para abstracción polimórfica de tipos comparables.
//!
//! Implementado de forma *zero-cost* para `[u8]`, `[u64]`, `str` y `String`.
//! La monomorfización del compilador elimina cualquier virtualización en runtime.

use crate::error::Result;
use crate::simd;

/// Trait que habilita el cálculo de distancia de Hamming para tipos de secuencia.
pub trait HammingTarget {
    /// Cardinalidad de la secuencia.
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }

    /// Distancia de Hamming frente a otra instancia del mismo tipo.
    fn hamming_distance(&self, other: &Self) -> Result<u64>;
}

impl HammingTarget for [u8] {
    #[inline(always)]
    fn len(&self) -> usize { <[u8]>::len(self) }

    #[inline(always)]
    fn hamming_distance(&self, other: &Self) -> Result<u64> {
        simd::hamming_distance_u8(self, other)
    }
}

impl HammingTarget for [u64] {
    #[inline(always)]
    fn len(&self) -> usize { <[u64]>::len(self) }

    #[inline(always)]
    fn hamming_distance(&self, other: &Self) -> Result<u64> {
        simd::popcount_xor(self, other)
    }
}

impl HammingTarget for str {
    #[inline(always)]
    fn len(&self) -> usize { str::len(self) }

    #[inline(always)]
    fn hamming_distance(&self, other: &Self) -> Result<u64> {
        simd::hamming_distance_u8(self.as_bytes(), other.as_bytes())
    }
}

impl HammingTarget for String {
    #[inline(always)]
    fn len(&self) -> usize { String::len(self) }

    #[inline(always)]
    fn hamming_distance(&self, other: &Self) -> Result<u64> {
        simd::hamming_distance_u8(self.as_bytes(), other.as_bytes())
    }
}
