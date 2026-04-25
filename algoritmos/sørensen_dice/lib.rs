use ahash::AHasher;
use rayon::prelude::*;
use std::hash::{Hash, Hasher};
use std::time::Instant;
use unicode_segmentation::UnicodeSegmentation;

// ==========================================
// Módulo: Shingler
// Generación eficiente de bigramas Unicode
// ==========================================
pub mod shingler {
    use super::*;

    /// Genera hashes de 64 bits a partir de bigramas Unicode (Grapheme Clusters).
    pub fn generate_shingles(text: &str) -> Vec<u64> {
        let graphemes: Vec<&str> = text.graphemes(true).collect();
        
        if graphemes.len() < 2 {
            let mut hasher = AHasher::default();
            text.hash(&mut hasher);
            return vec![hasher.finish()];
        }

        let mut hashes = Vec::with_capacity(graphemes.len() - 1);

        for window in graphemes.windows(2) {
            let mut hasher = AHasher::default();
            window[0].hash(&mut hasher);
            window[1].hash(&mut hasher);
            hashes.push(hasher.finish());
        }

        // Ordenar y deduplicar para garantizar operaciones de conjuntos limpias
        hashes.sort_unstable();
        hashes.dedup();
        
        hashes
    }
}

// ==========================================
// Módulo: Scoring
// Implementación optimizada de Sørensen-Dice
// ==========================================
pub mod scoring {
    /// Intersección SIMD/Branch-Optimized de dos arreglos ordenados.
    /// Zero-allocation (solo usa referencias a slices).
    #[inline(always)]
    fn intersect_sorted(a: &[u64], b: &[u64]) -> usize {
        let mut count = 0;
        let mut i = 0;
        let mut j = 0;

        // La CPU pre-fetchea estos arreglos contiguos de forma agresiva
        while i < a.len() && j < b.len() {
            // El compilador optimizará esto (frecuentemente eliminando ramas)
            if a[i] < b[j] {
                i += 1;
            } else if a[i] > b[j] {
                j += 1;
            } else {
                count += 1;
                i += 1;
                j += 1;
            }
        }
        count
    }

    /// Calcula el DSC con Lazy Evaluation / Pruning.
    /// Retorna None si es matemáticamente imposible alcanzar el threshold.
    #[inline(always)]
    pub fn dice_similarity(
        query: &[u64],
        record: &[u64],
        threshold: f64,
    ) -> Option<f64> {
        let len_q = query.len();
        let len_r = record.len();

        if len_q == 0 || len_r == 0 {
            return None;
        }

        // Pruning (Evaluación Perezosa): Max DSC posible ocurre si un conjunto es subconjunto del otro
        // Max Intersection = min(|A|, |B|)
        let max_possible_intersection = len_q.min(len_r) as f64;
        let max_possible_dsc = (2.0 * max_possible_intersection) / ((len_q + len_r) as f64);

        if max_possible_dsc < threshold {
            return None; // Ahorramos miles de ciclos de CPU saltando la intersección
        }

        let intersection_count = intersect_sorted(query, record);
        let score = (2.0 * intersection_count as f64) / ((len_q + len_r) as f64);

        if score >= threshold {
            Some(score)
        } else {
            None
        }
    }
}

// ==========================================
// Módulo: Index
// Gestión de datos Cache-Friendly (SoA Layout)
// ==========================================
pub mod index {
    use super::*;

    /// Estructura de Arreglos (SoA) para maximizar el CPU Prefetching.
    /// Mantiene metadatos separados del "Hot Path" de puntuación.
    pub struct CatalogIndex {
        pub ids: Vec<u32>,
        pub original_texts: Vec<String>,
        pub shingles_arrays: Vec<Vec<u64>>, // Hot data: Hashes contiguos
        pub cardinalities: Vec<usize>,      // Hot data: Pre-computación de |A|
    }

    impl CatalogIndex {
        pub fn new(capacity: usize) -> Self {
            Self {
                ids: Vec::with_capacity(capacity),
                original_texts: Vec::with_capacity(capacity),
                shingles_arrays: Vec::with_capacity(capacity),
                cardinalities: Vec::with_capacity(capacity),
            }
        }

        pub fn insert(&mut self, id: u32, text: &str) {
            let shingles = shingler::generate_shingles(text);
            self.cardinalities.push(shingles.len());
            self.shingles_arrays.push(shingles);
            self.original_texts.push(text.to_string());
            self.ids.push(id);
        }

        /// Búsqueda concurrente utilizando Rayon ThreadPool
        pub fn search(&self, query_text: &str, threshold: f64) -> Vec<(u32, &str, f64)> {
            let start_time = Instant::now();
            let query_shingles = shingler::generate_shingles(query_text);

            // zip() itera múltiples slices en paralelo conservando la afinidad de caché
            let mut results: Vec<(u32, &str, f64)> = self.shingles_arrays
                .par_iter()
                .zip(self.ids.par_iter())
                .zip(self.original_texts.par_iter())
                .filter_map(|((record_shingles, id), text)| {
                    // Llamada Zero-Allocation
                    match scoring::dice_similarity(&query_shingles, record_shingles, threshold) {
                        Some(score) => Some((*id, text.as_str(), score)),
                        None => None,
                    }
                })
                .collect();

            // Ordenar por relevancia descendente
            results.sort_unstable_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

            let elapsed = start_time.elapsed();
            println!("[LOG] Latencia de búsqueda: {} microsegundos", elapsed.as_micros());

            results
        }
    }
}
