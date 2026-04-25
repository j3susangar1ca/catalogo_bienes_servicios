//! Motor SIMD portable para cálculo de Distancia de Hamming.

use crate::error::{Result, HammingError};

// =============================================================================
// POPCOUNT XOR para arrays de u64 (Bit-Flags)
// =============================================================================

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

// Mantener las funciones _simd para compatibilidad si se usan en otros lugares
#[inline(always)]
pub fn popcount_xor_simd<const LANES: usize>(a: &[u64], b: &[u64]) -> Result<u64> {
    popcount_xor(a, b)
}

#[inline(always)]
pub fn hamming_distance_u8_simd<const LANES: usize>(a: &[u8], b: &[u8]) -> Result<u64> {
    hamming_distance_u8(a, b)
}
