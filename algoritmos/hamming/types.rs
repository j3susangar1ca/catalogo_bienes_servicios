//! Tipos de dominio especializados para el catálogo.
//!
//! Incluye `BitMap` para atributos binarios y `IdentityCode` para SKUs,
//! ambos diseñados para maximizar el throughput SIMD y la seguridad de tipos.

use crate::simd;
use crate::error::Result;

// =============================================================================
// Alineación de Memoria
// =============================================================================

/// Bloque de 32 bytes (4 × u64) alineado a 32 bytes.
/// 
/// Esta alineación garantiza que las cargas SIMD (`vmovdqa` en x86_64)
/// nunca crucen límites de caché de 32 bytes, maximizando el ancho de banda.
#[repr(align(32))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct AlignedU64x4(pub [u64; 4]);

// =============================================================================
// Newtypes de Dominio
// =============================================================================

/// Código identificador de producto (SKU).
/// 
/// `IdentityCode` es un *newtype* sobre `String` que previene confusiones
/// de tipos en el catálogo (e.g., no se puede pasar un SKU donde se espera un BitMap).
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct IdentityCode(pub String);

/// Mapa de bits para atributos técnicos de producto.
/// 
/// Almacena los bits en bloques de 32 bytes alineados, optimizando
/// las operaciones `popcount(xor)` del motor SIMD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitMap {
    blocks: Vec<AlignedU64x4>,
    /// Número lógico de u64s (el último bloque puede estar parcialmente utilizado).
    len: usize,
}

impl BitMap {
    /// Crea un `BitMap` inicializado a cero con `u64_count` palabras de 64 bits.
    pub fn zeros(u64_count: usize) -> Self {
        let blocks_needed = (u64_count + 3) / 4;
        Self {
            blocks: vec![AlignedU64x4([0; 4]); blocks_needed],
            len: u64_count,
        }
    }

    /// Construye un `BitMap` a partir de un slice de `u64`.
    pub fn from_u64_slice(data: &[u64]) -> Self {
        let mut blocks = Vec::with_capacity((data.len() + 3) / 4);
        for chunk in data.chunks(4) {
            let mut arr = [0u64; 4];
            arr[..chunk.len()].copy_from_slice(chunk);
            blocks.push(AlignedU64x4(arr));
        }
        Self { blocks, len: data.len() }
    }

    /// Expone los datos como un slice plano de `u64`.
    /// 
    /// # SAFETY
    /// `AlignedU64x4` es `#[repr(align(32))]` y contiene únicamente `[u64; 4]`.
    /// Dado que `Vec` garantiza contigüidad, es seguro "aplanar" los bloques.
    /// Solo se exponen `self.len` elementos para respetar la cardinalidad lógica.
    pub fn as_u64_slice(&self) -> &[u64] {
        if self.blocks.is_empty() {
            return &[];
        }
        let max_len = self.blocks.len() * 4;
        let safe_len = self.len.min(max_len);
        unsafe {
            core::slice::from_raw_parts(
                self.blocks.as_ptr() as *const u64,
                safe_len
            )
        }
    }

    /// Construye un `BitMap` aleatorio para benchmarking y tests.
    #[cfg(test)]
    pub fn random(u64_count: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let data: Vec<u64> = (0..u64_count).map(|_| rng.gen()).collect();
        Self::from_u64_slice(&data)
    }

    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Calcula la distancia de Hamming contra otro `BitMap`.
    #[inline(always)]
    pub fn hamming_distance(&self, other: &Self) -> Result<u64> {
        simd::popcount_xor(self.as_u64_slice(), other.as_u64_slice())
    }
}
