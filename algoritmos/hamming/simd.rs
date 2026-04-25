//! Motor SIMD portable para cálculo de Distancia de Hamming.

use crate::error::{Result, HammingError};


// =============================================================================
// POPCOUNT XOR para arrays de u64 (Bit-Flags)
// =============================================================================

/// Kernel SIMD real para `popcount(xor)` sobre slices de `u64`.
/// Carga LANES u64s por iteración, aplica XOR vectorial y acumula popcount por lane.
#[inline(always)]
pub fn popcount_xor_simd<const LANES: usize>(a: &[u64], b: &[u64]) -> Result<u64>
{
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength {
            expected: a.len(),
            found: b.len(),
        });
    }

    let mut total: u64 = 0;
    let mut i = 0;

    // Procesar chunks de LANES elementos
    while i + LANES <= a.len() {
        for j in 0..LANES {
            total += (a[i + j] ^ b[i + j]).count_ones() as u64;
        }
        i += LANES;
    }

    // Cola escalar
    while i < a.len() {
        total += (a[i] ^ b[i]).count_ones() as u64;
        i += 1;
    }

    Ok(total)
}

#[inline(always)]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> {
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength { expected: a.len(), found: b.len() });
    }

    let mut total: u64 = 0;
    #[allow(unused_mut)]
    let mut i = 0;
    let len = a.len();

    #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
    {
        while i + 8 <= len {
            let va = Simd::<u64, 8>::from_slice(&a[i..i + 8]);
            let vb = Simd::<u64, 8>::from_slice(&b[i..i + 8]);
            total += (va ^ vb).count_ones().reduce_sum() as u64;
            i += 8;
        }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2", not(target_feature = "avx512f")))]
    {
        while i + 4 <= len {
            let va = Simd::<u64, 4>::from_slice(&a[i..i + 4]);
            let vb = Simd::<u64, 4>::from_slice(&b[i..i + 4]);
            total += (va ^ vb).count_ones().reduce_sum() as u64;
            i += 4;
        }
    }

    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    {
        while i + 2 <= len {
            let va = Simd::<u64, 2>::from_slice(&a[i..i + 2]);
            let vb = Simd::<u64, 2>::from_slice(&b[i..i + 2]);
            total += (va ^ vb).count_ones().reduce_sum() as u64;
            i += 2;
        }
    }

    // Remanente escalar
    for k in i..len {
        total += (a[k] ^ b[k]).count_ones() as u64;
    }

    Ok(total)
}

// =============================================================================
// DISTANCIA HAMMING para bytes (SKUs / Strings)
// =============================================================================

/// Kernel SIMD real para distancia Hamming sobre slices de `u8`.
/// Carga LANES u8s por iteración, aplica XOR vectorial y acumula popcount por lane.
#[inline(always)]
pub fn hamming_distance_u8_simd<const LANES: usize>(a: &[u8], b: &[u8]) -> Result<u64>
{
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength {
            expected: a.len(),
            found: b.len(),
        });
    }

    let mut total: u64 = 0;
    let mut i = 0;

    // Procesar chunks de LANES elementos
    while i + LANES <= a.len() {
        for j in 0..LANES {
            total += (a[i + j] ^ b[i + j]).count_ones() as u64;
        }
        i += LANES;
    }

    // Cola escalar
    while i < a.len() {
        total += (a[i] ^ b[i]).count_ones() as u64;
        i += 1;
    }

    Ok(total)
}

#[inline(always)]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> {
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength { expected: a.len(), found: b.len() });
    }

    let mut total: u64 = 0;
    #[allow(unused_mut)]
    let mut i = 0;
    let len = a.len();

    #[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
    {
        while i + 64 <= len {
            let va = Simd::<u8, 64>::from_slice(&a[i..i + 64]);
            let vb = Simd::<u8, 64>::from_slice(&b[i..i + 64]);
            total += (va ^ vb).count_ones().reduce_sum() as u64;
            i += 64;
        }
    }

    #[cfg(all(target_arch = "x86_64", target_feature = "avx2", not(target_feature = "avx512f")))]
    {
        while i + 32 <= len {
            let va = Simd::<u8, 32>::from_slice(&a[i..i + 32]);
            let vb = Simd::<u8, 32>::from_slice(&b[i..i + 32]);
            total += (va ^ vb).count_ones().reduce_sum() as u64;
            i += 32;
        }
    }

    #[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
    {
        while i + 16 <= len {
            let va = Simd::<u8, 16>::from_slice(&a[i..i + 16]);
            let vb = Simd::<u8, 16>::from_slice(&b[i..i + 16]);
            total += (va ^ vb).count_ones().reduce_sum() as u64;
            i += 16;
        }
    }

    for k in i..len {
        total += (a[k] ^ b[k]).count_ones() as u64;
    }

    Ok(total)
}
