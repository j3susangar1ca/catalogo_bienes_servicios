// ============================================================
//  MODULE: vector_math
//  Producto Punto y Norma L2 optimizados con auto-vectorización
//  y soporte explícito para SIMD via `wide`.
// ============================================================

use wide::f32x8;

// ------------------------------------------------------------
//  CONSTANTES DE LAYOUT
// ------------------------------------------------------------
/// Alineación requerida para cargas AVX2 (32 bytes = 8 × f32).
pub const ALIGN: usize = 32;
/// Dimensión estándar de embeddings BERT-base.
pub const DIM: usize = 768;

// ------------------------------------------------------------
//  ALIGNED VECTOR BUFFER
// ------------------------------------------------------------
/// Buffer de floats alineado a 32 bytes en HEAP, listo para SIMD.
/// Garantiza que el puntero a los datos caiga en un boundary de 32 bytes.
pub struct AlignedVec {
    // Usamos f32x8 para garantizar que el allocator use alineación de 32 bytes.
    data: Vec<f32x8>,
}

impl AlignedVec {
    /// Crea un nuevo buffer de `dim` elementos, inicializado a cero.
    pub fn zeros(dim: usize) -> Self {
        assert_eq!(dim % 8, 0, "dim debe ser múltiplo de 8 para alineación f32x8");
        Self { data: vec![f32x8::ZERO; dim / 8] }
    }

    /// Crea desde un slice copiando los valores.
    pub fn from_slice(src: &[f32]) -> Self {
        let mut v = Self::zeros(src.len());
        v.as_slice_mut().copy_from_slice(src);
        v
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[f32] {
        let ptr = self.data.as_ptr() as *const f32;
        let len = self.data.len() * 8;
        unsafe { std::slice::from_raw_parts(ptr, len) }
    }

    #[inline(always)]
    pub fn as_slice_mut(&mut self) -> &mut [f32] {
        let ptr = self.data.as_mut_ptr() as *mut f32;
        let len = self.data.len() * 8;
        unsafe { std::slice::from_raw_parts_mut(ptr, len) }
    }
}

// ------------------------------------------------------------
//  PRODUCTO PUNTO — ruta SIMD explícita (wide f32x8)
// ------------------------------------------------------------
/// Calcula `A · B` procesando 8 dimensiones por ciclo (AVX2 width).
/// El compilador emite instrucciones `vdpps` / `vfmadd231ps` en x86-64.
///
/// # Panics
/// Si `a.len() != b.len()` o la longitud no es múltiplo de 8.
#[inline]
pub fn dot_simd(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "dot_simd: longitudes distintas");
    debug_assert_eq!(a.len() % 8, 0, "dot_simd: longitud debe ser múltiplo de 8");

    let mut acc = f32x8::ZERO;
    let chunks_a = a.chunks_exact(8);
    let chunks_b = b.chunks_exact(8);

    for (ca, cb) in chunks_a.zip(chunks_b) {
        // SAFETY: chunks_exact garantiza slices de exactamente 8 elementos.
        let va = f32x8::from([ca[0], ca[1], ca[2], ca[3], ca[4], ca[5], ca[6], ca[7]]);
        let vb = f32x8::from([cb[0], cb[1], cb[2], cb[3], cb[4], cb[5], cb[6], cb[7]]);
        acc += va * vb;
    }

    // Reducción horizontal: suma los 8 lanes
    let arr: [f32; 8] = acc.into();
    arr.iter().sum()
}

/// Versión escalar con auto-vectorización habilitada (fallback portable).
/// El compilador puede vectorizarla en targets sin `wide` disponible.
#[inline]
pub fn dot_scalar(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

// ------------------------------------------------------------
//  NORMA L2
// ------------------------------------------------------------
/// Retorna ‖v‖₂ = √(v · v).
#[inline]
pub fn l2_norm(v: &[f32]) -> f32 {
    dot_simd(v, v).sqrt()
}

// ------------------------------------------------------------
//  NORMALIZACIÓN IN-PLACE
// ------------------------------------------------------------
/// Divide cada componente por ‖v‖₂, dejando ‖v‖₂ = 1.
/// Después de esto, similitud coseno ≡ producto punto.
pub fn normalize_inplace(v: &mut [f32]) {
    let norm = l2_norm(v);
    if norm > 1e-10 {
        let inv = 1.0 / norm;
        v.iter_mut().for_each(|x| *x *= inv);
    }
}

// ------------------------------------------------------------
//  COSINE SIMILARITY (raw, sin pre-normalización)
// ------------------------------------------------------------
/// Similitud de coseno completa: (A·B) / (‖A‖·‖B‖).
/// Úsala solo cuando los vectores NO están pre-normalizados.
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot   = dot_simd(a, b);
    let norm_a = l2_norm(a);
    let norm_b = l2_norm(b);
    if norm_a < 1e-10 || norm_b < 1e-10 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ─────────────────────────────────────────────────────────────
//  TESTS UNITARIOS
// ─────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn pad8(v: Vec<f32>) -> Vec<f32> {
        let mut v = v;
        while v.len() % 8 != 0 { v.push(0.0); }
        v
    }

    #[test]
    fn test_dot_identical_unit_vectors() {
        let a = pad8(vec![1.0, 0.0, 0.0]);
        let b = pad8(vec![1.0, 0.0, 0.0]);
        assert!((dot_simd(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = pad8(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let b = pad8(vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(cosine_similarity(&a, &b).abs() < 1e-6);
    }

    #[test]
    fn test_normalize() {
        let mut v = pad8(vec![3.0, 4.0]);
        normalize_inplace(&mut v);
        assert!((l2_norm(&v) - 1.0).abs() < 1e-6);
    }
}
