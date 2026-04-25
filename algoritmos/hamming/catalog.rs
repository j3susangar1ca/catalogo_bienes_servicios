//! Índice de catálogo de 64K registros con búsqueda por proximidad Hamming.

use crate::types::{BitMap, IdentityCode};
use crate::error::Result;

/// Registro individual del catálogo, alineado a 32 bytes para optimizar prefetch.
#[repr(align(32))]
#[derive(Clone, Debug)]
pub struct CatalogRecord {
    pub sku: IdentityCode,
    pub attributes: BitMap,
}

/// Índice de catálogo que organiza hasta 64,000 registros para búsquedas
/// de proximidad por distancia de Hamming.
pub struct CatalogIndex {
    records: Vec<CatalogRecord>,
}

impl CatalogIndex {
    /// Crea un índice vacío con capacidad pre-reservada.
    #[inline(always)]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            records: Vec::with_capacity(cap),
        }
    }

    /// Inserta un nuevo registro en el catálogo.
    #[inline(always)]
    pub fn insert(&mut self, sku: IdentityCode, attributes: BitMap) {
        self.records.push(CatalogRecord { sku, attributes });
    }

    pub fn len(&self) -> usize { self.records.len() }
    pub fn is_empty(&self) -> bool { self.records.is_empty() }

    // =========================================================================
    // Bit-Flag Comparator
    // =========================================================================

    /// Filtra productos cuyos atributos disten `<= max_distance` del target.
    /// 
    /// Procesa **todos** los registros para mantener un perfil de ejecución
    /// relativamente constante (evita early-exit dependiente de datos).
    #[inline(always)]
    pub fn find_by_attribute_distance(
        &self,
        target: &BitMap,
        max_distance: u64,
    ) -> Vec<&CatalogRecord> {
        let mut matches = Vec::new();
        // Pre-reserva para evitar reallocations durante la búsqueda.
        matches.reserve(256);

        for record in &self.records {
            // Cálculo obligatorio: mantiene el tiempo de ejecución independiente
            // de la coincidencia (sin early-exit en el loop crítico).
            let dist = record.attributes.hamming_distance(target);

            // La ramificación aquí es sobre el umbral, no sobre el contenido de los datos.
            if let Ok(d) = dist {
                if d <= max_distance {
                    matches.push(record);
                }
            }
        }
        matches
    }

    // =========================================================================
    // SKU Validator (Single-Digit Error Detection)
    // =========================================================================

    /// Detecta SKUs existentes que disten exactamente 1 del candidato.
    /// 
    /// Útil para detectar errores de tipeo de un solo carácter en entrada manual.
    /// Solo compara SKUs de **igual longitud** (distancia de Hamming está definida
    /// para vectores de igual cardinalidad).
    #[inline(always)]
    pub fn find_sku_typos(&self, candidate: &IdentityCode) -> Vec<&CatalogRecord> {
        let mut matches = Vec::new();
        let candidate_bytes = candidate.0.as_bytes();
        let cand_len = candidate_bytes.len();

        for record in &self.records {
            let sku_bytes = record.sku.0.as_bytes();
            // Filtrado por longitud: Hamming requiere len(A) == len(B).
            if sku_bytes.len() == cand_len {
                // SAFETY: Longitudes verificadas iguales; `hamming_distance_u8` no
                // retornará `IncompatibleLength` en este contexto.
                if let Ok(1) = crate::simd::hamming_distance_u8(sku_bytes, candidate_bytes) {
                    matches.push(record);
                }
            }
        }
        matches
    }
}
