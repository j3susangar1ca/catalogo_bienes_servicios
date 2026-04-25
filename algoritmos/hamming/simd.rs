//! Motor SIMD portable para cálculo de Distancia de Hamming.
//!
//! Implementa `d_H(x, y) = popcount(x XOR y)` utilizando `std::simd`.
//! La selección del ancho de vector (LANES) se realiza en tiempo de compilación
//! mediante `cfg` flags, aprovechando AVX-512 (8×u64), AVX2 (4×u64) o NEON (2×u64).

use std::simd::{Simd, LaneCount, SupportedLaneCount};
use std::simd::num::SimdUint;
use crate::error::{Result, HammingError};

// =============================================================================
// POPCOUNT XOR para arrays de u64 (Bit-Flags)
// =============================================================================

/// Kernel SIMD genérico para `popcount(xor)` sobre slices de `u64`.
#[inline(always)]
pub fn popcount_xor_simd<const LANES: usize>(a: &[u64], b: &[u64]) -> Result<u64>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength {
            expected: a.len(),
            found: b.len(),
        });
    }

    let mut total: u64 = 0;
    let mut i = 0;
    let len = a.len();

    while i + LANES <= len {
        let va = Simd::<u64, LANES>::from_slice(&a[i..i + LANES]);
        let vb = Simd::<u64, LANES>::from_slice(&b[i..i + LANES]);
        total += (va ^ vb).count_ones().reduce_sum() as u64;
        i += LANES;
    }

    // Remanente escalar
    for k in i..len {
        total += (a[k] ^ b[k]).count_ones() as u64;
    }

    Ok(total)
}

// Selección de ancho vectorial en tiempo de compilación basada en target features.
/// AVX-512F: 512 bits = 8 × u64.
#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
#[inline(always)]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> {
    popcount_xor_simd::<8>(a, b)
}

/// AVX2: 256 bits = 4 × u64.
#[cfg(all(
    target_arch = "x86_64",
    target_feature = "avx2",
    not(target_feature = "avx512f")
))]
#[inline(always)]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> {
    popcount_xor_simd::<4>(a, b)
}

/// ARM NEON: 128 bits = 2 × u64.
#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
#[inline(always)]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> {
    popcount_xor_simd::<2>(a, b)
}

/// Fallback genérico.
#[cfg(not(any(
    all(target_arch = "x86_64", target_feature = "avx2"),
    all(target_arch = "x86_64", target_feature = "avx512f"),
    all(target_arch = "aarch64", target_feature = "neon")
)))]
#[inline(always)]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> {
    popcount_xor_simd::<2>(a, b)
}

// =============================================================================
// DISTANCIA HAMMING para bytes (SKUs / Strings)
// =============================================================================

/// Kernel SIMD genérico para distancia de Hamming sobre bytes (`u8`).
#[inline(always)]
pub fn hamming_distance_u8_simd<const LANES: usize>(a: &[u8], b: &[u8]) -> Result<u64>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength {
            expected: a.len(),
            found: b.len(),
        });
    }

    let mut total: u64 = 0;
    let mut i = 0;
    let len = a.len();

    while i + LANES <= len {
        let va = Simd::<u8, LANES>::from_slice(&a[i..i + LANES]);
        let vb = Simd::<u8, LANES>::from_slice(&b[i..i + LANES]);
        total += (va ^ vb).count_ones().reduce_sum() as u64;
        i += LANES;
    }

    for k in i..len {
        total += (a[k] ^ b[k]).count_ones() as u64;
    }

    Ok(total)
}

/// AVX-512F: 512 bits = 64 × u8.
#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
#[inline(always)]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> {
    hamming_distance_u8_simd::<64>(a, b)
}

/// AVX2: 256 bits = 32 × u8.
#[cfg(all(
    target_arch = "x86_64",
    target_feature = "avx2",
    not(target_feature = "avx512f")
))]
#[inline(always)]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> {
    hamming_distance_u8_simd::<32>(a, b)
}

/// ARM NEON: 128 bits = 16 × u8.
#[cfg(all(target_arch = "aarch64", target_feature = "neon"))]
#[inline(always)]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> {
    hamming_distance_u8_simd::<16>(a, b)
}

/// Fallback genérico.
#[cfg(not(any(
    all(target_arch = "x86_64", target_feature = "avx2"),
    all(target_arch = "x86_64", target_feature = "avx512f"),
    all(target_arch = "aarch64", target_feature = "neon")
)))]
#[inline(always)]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> {
    hamming_distance_u8_simd::<16>(a, b)
}
