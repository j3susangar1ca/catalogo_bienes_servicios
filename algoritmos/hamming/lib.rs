//! # Hamming HPC Engine
//! 
//! Motor ultra-eficiente para cálculo de Distancia de Hamming orientado a catálogos
//! de 64,000 registros. Incluye vectorización SIMD portable (AVX2/AVX-512/NEON),
//! alineación de memoria a 32 bytes, y abstracciones de costo cero.

pub mod error;
pub mod simd;
pub mod types;
pub mod catalog;
pub mod hamming_trait;

pub use error::{HammingError, Result};
pub use types::{BitMap, IdentityCode, AlignedU64x4};
pub use catalog::{CatalogIndex, CatalogRecord};
pub use hamming_trait::HammingTarget;
