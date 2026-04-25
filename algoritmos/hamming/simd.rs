//! Motor SIMD portable para cálculo de Distancia de Hamming.

use std::simd::{Simd, SimdUint};
use crate::error::{Result, HammingError};

// Helper trait-like bounds if they are not in scope
use std::simd::lane::LaneCount;
use std::simd::lane::SupportedLaneCount;

#[inline(always)]
pub fn popcount_xor_simd<const LANES: usize>(a: &[u64], b: &[u64]) -> Result<u64>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength { expected: a.len(), found: b.len() });
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
    for k in i..len {
        total += (a[k] ^ b[k]).count_ones() as u64;
    }
    Ok(total)
}

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> { popcount_xor_simd::<8>(a, b) }

#[cfg(all(target_arch = "x86_64", target_feature = "avx2", not(target_feature = "avx512f")))]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> { popcount_xor_simd::<4>(a, b) }

#[cfg(not(any(all(target_arch = "x86_64", target_feature = "avx2"), all(target_arch = "x86_64", target_feature = "avx512f"))))]
pub fn popcount_xor(a: &[u64], b: &[u64]) -> Result<u64> { popcount_xor_simd::<2>(a, b) }

#[inline(always)]
pub fn hamming_distance_u8_simd<const LANES: usize>(a: &[u8], b: &[u8]) -> Result<u64>
where
    LaneCount<LANES>: SupportedLaneCount,
{
    if a.len() != b.len() {
        return Err(HammingError::IncompatibleLength { expected: a.len(), found: b.len() });
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

#[cfg(all(target_arch = "x86_64", target_feature = "avx512f"))]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> { hamming_distance_u8_simd::<64>(a, b) }

#[cfg(all(target_arch = "x86_64", target_feature = "avx2", not(target_feature = "avx512f")))]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> { hamming_distance_u8_simd::<32>(a, b) }

#[cfg(not(any(all(target_arch = "x86_64", target_feature = "avx2"), all(target_arch = "x86_64", target_feature = "avx512f"))))]
pub fn hamming_distance_u8(a: &[u8], b: &[u8]) -> Result<u64> { hamming_distance_u8_simd::<16>(a, b) }
