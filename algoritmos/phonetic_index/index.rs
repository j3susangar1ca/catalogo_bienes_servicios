//! Índice fonético inverso: `PhoneticCode → Vec<RecordId>`.
//!
//! - Construcción paralela con **Rayon**.
//! - Hasher rápido: **FxHash** (`rustc-hash`).
//! - Búsqueda O(1) tras la indexación.
//! - Búsqueda fuzzy con variantes a distancia de edición ≤ 1.

use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::phonetic_core::{Code, DoubleMetaphone, PhoneticEncoder};

// ─── Tipos ──────────────────────────────────────────────────

pub type RecordId = u32;

// ─── PhoneticIndex ──────────────────────────────────────────

pub struct PhoneticIndex {
    primary_map: FxHashMap<Code, Vec<RecordId>>,
    secondary_map: FxHashMap<Code, Vec<RecordId>>,
    encoder: DoubleMetaphone,
    total_records: usize,
}

impl PhoneticIndex {
    /// Construye el índice en paralelo a partir de un catálogo `(id, nombre)`.
    pub fn build(catalog: &[(RecordId, String)]) -> Self {
        let encoder = DoubleMetaphone::new();

        // ── Codificación paralela ─────────────────────────
        let encoded: Vec<(RecordId, Code, Code)> = catalog
            .par_iter()
            .map(|(id, name)| {
                let (pri, sec) = encoder.encode_double(name);
                (*id, pri, sec)
            })
            .collect();

        // ── Construcción de mapas ─────────────────────────
        let mut primary_map: FxHashMap<Code, Vec<RecordId>> = FxHashMap::default();
        let mut secondary_map: FxHashMap<Code, Vec<RecordId>> = FxHashMap::default();

        for (id, pri, sec) in &encoded {
            primary_map.entry(*pri).or_default().push(*id);
            secondary_map.entry(*sec).or_default().push(*id);
        }

        let total_records = catalog.len();

        Self {
            primary_map,
            secondary_map,
            encoder,
            total_records,
        }
    }

    /// Búsqueda fonética exacta: compara los 4 cruces posibles
    /// (pri/sec de la consulta × pri/sec del índice).
    pub fn search(&self, query: &str) -> Vec<RecordId> {
        let (pri, sec) = self.encoder.encode_double(query);
        let mut seen = FxHashSet::default();
        let mut results = Vec::new();

        self.collect_matches(&pri, &mut seen, &mut results);
        if sec != pri {
            self.collect_matches(&sec, &mut seen, &mut results);
        }

        results
    }

    /// Búsqueda fuzzy: genera variantes del código fonético
    /// (inserción, borrado, sustitución, transposición) y busca cada una.
    /// Devuelve `(RecordId, distancia)`.
    pub fn fuzzy_search(&self, query: &str, max_distance: usize) -> Vec<(RecordId, usize)> {
        let (pri, sec) = self.encoder.encode_double(query);

        // Primero: coincidencias exactas
        let exact = self.search(query);
        if !exact.is_empty() {
            return exact.into_iter().map(|id| (id, 0)).collect();
        }

        let mut seen_ids: FxHashSet<RecordId> = FxHashSet::default();
        let mut seen_codes: FxHashSet<Code> = FxHashSet::default();
        let mut results: Vec<(RecordId, usize)> = Vec::new();

        for base in [pri, sec] {
            let variants = generate_variants(&base, max_distance);
            for (variant, dist) in variants {
                if !seen_codes.insert(variant) {
                    continue;
                }
                if let Some(ids) = self.primary_map.get(&variant) {
                    for &id in ids {
                        if seen_ids.insert(id) {
                            results.push((id, dist));
                        }
                    }
                }
                if let Some(ids) = self.secondary_map.get(&variant) {
                    for &id in ids {
                        if seen_ids.insert(id) {
                            results.push((id, dist));
                        }
                    }
                }
            }
        }

        results.sort_by_key(|&(_, d)| d);
        results
    }

    /// Número de claves únicas en el mapa primario.
    pub fn primary_key_count(&self) -> usize {
        self.primary_map.len()
    }

    /// Número de claves únicas en el mapa secundario.
    pub fn secondary_key_count(&self) -> usize {
        self.secondary_map.len()
    }

    pub fn total_records(&self) -> usize {
        self.total_records
    }

    // ── Privados ──────────────────────────────────────────

    fn collect_matches(
        &self,
        code: &Code,
        seen: &mut FxHashSet<RecordId>,
        results: &mut Vec<RecordId>,
    ) {
        if let Some(ids) = self.primary_map.get(code) {
            for &id in ids {
                if seen.insert(id) {
                    results.push(id);
                }
            }
        }
        if let Some(ids) = self.secondary_map.get(code) {
            for &id in ids {
                if seen.insert(id) {
                    results.push(id);
                }
            }
        }
    }
}

// ─── Generación de variantes fuzzy ──────────────────────────

/// Genera todas las variantes de un código fonético con distancia de edición ≤ `max_d`.
/// Incluye: borrados, sustituciones, inserciones y transposiciones.
fn generate_variants(code: &Code, max_d: usize) -> Vec<(Code, usize)> {
    if max_d == 0 {
        return vec![];
    }

    let chars: Vec<char> = code.chars().collect();
    let mut variants = Vec::with_capacity(400);

    // Borrados
    for i in 0..chars.len() {
        let mut v = Code::new();
        for (j, &c) in chars.iter().enumerate() {
            if j != i {
                let _ = v.try_push(c);
            }
        }
        variants.push((v, 1));
    }

    // Sustituciones
    for i in 0..chars.len() {
        for r in b'A'..=b'Z' {
            let rch = r as char;
            if rch == chars[i] {
                continue;
            }
            let mut v = Code::new();
            for (j, &c) in chars.iter().enumerate() {
                let _ = v.try_push(if j == i { rch } else { c });
            }
            variants.push((v, 1));
        }
    }

    // Inserciones
    for i in 0..=chars.len() {
        for ins in b'A'..=b'Z' {
            let insch = ins as char;
            let mut v = Code::new();
            for (j, &c) in chars.iter().enumerate() {
                if j == i {
                    let _ = v.try_push(insch);
                }
                let _ = v.try_push(c);
            }
            if i == chars.len() {
                let _ = v.try_push(insch);
            }
            variants.push((v, 1));
        }
    }

    // Transposiciones
    for i in 0..chars.len().saturating_sub(1) {
        let mut v = Code::new();
        for (j, &c) in chars.iter().enumerate() {
            if j == i {
                let _ = v.try_push(chars[i + 1]);
            } else if j == i + 1 {
                let _ = v.try_push(chars[i]);
            } else {
                let _ = v.try_push(c);
            }
        }
        variants.push((v, 1));
    }

    variants
}
