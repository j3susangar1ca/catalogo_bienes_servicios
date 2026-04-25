//! Motor de Similitud de Jaccard para Catálogos de 64k Registros
//!
//! Este motor implementa el cálculo exacto y aproximado (MinHash) de la
//! similitud de Jaccard sobre bolsas de palabras, orientado a baja latencia
//! y alto rendimiento en CPU.
//!
//! Características:
//! - Tokenización con normalización Unicode NFD y filtro de *stop‑words*.
//! - Hashing rápido (AHash) y representación como vectores ordenados de `u64`.
//! - Intersección exacta con marcha de dos punteros (O(n+m) sin reservas de memoria).
//! - Firmas MinHash para estimación en O(1) con respecto al tamaño de los conjuntos.
//! - Paralelización con Rayon.
//! - Diseño genérico mediante el trait `JaccardIndex<T>`.
//! - Tipos seguros (`SimilarityScore`) y documentación exhaustiva con complejidades.

use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use ahash::{AHasher, RandomState};
use rayon::prelude::*;
use unicode_normalization::UnicodeNormalization;

// ---------------------------------------------------------------------------
// Módulo de tokenización
// ---------------------------------------------------------------------------
pub mod tokenizer {
    use super::*;

    /// Lista estática de *stop‑words* en español.
    static STOP_WORDS: &[&str] = &[
        "el", "la", "los", "las", "un", "una", "unos", "unas", "lo", "al", "del", "a", "e", "o",
        "y", "de", "en", "con", "por", "para", "que", "es", "su", "se", "no", "si", "le", "yo",
        "tu", "él", "ella", "nos", "vos", "ellos", "ellas",
    ];

    /// Conjunto de *stop‑words* para búsqueda O(1).
    lazy_static::lazy_static! {
        static ref STOP_SET: HashSet<&'static str> = {
            let mut set = HashSet::with_capacity(STOP_WORDS.len());
            for w in STOP_WORDS {
                set.insert(*w);
            }
            set
        };
    }

    /// Convierte una cadena de texto en un vector de *hashes* de 64 bits
    /// ordenado, listo para comparaciones Jaccard.
    ///
    /// Pipeline:
    /// 1. Normalización Unicode NFD (una asignación de `String`).
    /// 2. Segmentación por delimitadores alfanuméricos (zero‑copy sobre la cadena normalizada).
    /// 3. Filtro de *stop‑words* y tokens vacíos.
    /// 4. Hashing de cada token con AHash → `u64`.
    /// 5. Ordenación del vector resultante.
    ///
    /// Complejidad temporal: O(n log n) dominada por la ordenación,
    /// donde n es el número de tokens.
    /// Complejidad espacial: O(n) para los hashes más la cadena normalizada.
    pub fn tokenize_and_hash(input: &str) -> Vec<u64> {
        // Normalización NFD (la forma canónica descompuesta)
        let normalized: String = input.nfd().collect();
        // Hasher con semilla aleatoria (alta calidad y rápida)
        let hash_builder = RandomState::default();

        let mut hashes: Vec<u64> = normalized
            .split(|c: char| !c.is_alphanumeric())
            .filter(|token| !token.is_empty() && !STOP_SET.contains(token))
            .map(|token| hash_builder.hash_one(token))
            .collect();

        hashes.sort_unstable();
        hashes
    }
}

// ---------------------------------------------------------------------------
// Módulo de operaciones sobre conjuntos (set_ops)
// ---------------------------------------------------------------------------
pub mod set_ops {
    /// Calcula el tamaño de la intersección de dos *slices* **ordenados**
    /// de `u64` sin realizar asignaciones de memoria.
    ///
    /// Emplea la técnica de **two‑way merge** (dos punteros) que recorre
    /// ambos *slices* en tiempo lineal O(n + m) y espacio O(1).
    ///
    /// # Panics
    /// No produce pánicos. Si alguno de los *slices* no está ordenado,
    /// el resultado es impredecible pero no inseguro.
    pub fn intersection_count(a: &[u64], b: &[u64]) -> usize {
        let mut i = 0;
        let mut j = 0;
        let mut count = 0;
        while i < a.len() && j < b.len() {
            match a[i].cmp(&b[j]) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    count += 1;
                    i += 1;
                    j += 1;
                }
            }
        }
        count
    }

    /// Calcula el coeficiente de Jaccard exacto entre dos conjuntos
    /// representados como *slices* ordenados de `u64`.
    ///
    /// J(A, B) = |A ∩ B| / |A ∪ B|
    ///
    /// Si ambos conjuntos están vacíos, la similitud es 1.0 por convención.
    pub fn jaccard(a: &[u64], b: &[u64]) -> f32 {
        let inter = intersection_count(a, b);
        let union = a.len() + b.len() - inter;
        if union == 0 {
            1.0
        } else {
            inter as f32 / union as f32
        }
    }
}

// ---------------------------------------------------------------------------
// Módulo MinHash
// ---------------------------------------------------------------------------
pub mod minhash {
    use super::*;

    /// Generador de firmas MinHash con `k` funciones hash.
    pub struct MinHasher {
        seeds: Vec<u64>,       // semillas para las k funciones
    }

    impl MinHasher {
        /// Construye un nuevo `MinHasher` con `k` funciones.
        ///
        /// Las semillas se generan de forma determinista a partir de una semilla
        /// fija y un LCG para garantizar reproducibilidad.
        pub fn new(k: usize) -> Self {
            let mut seeds = Vec::with_capacity(k);
            let mut state = 0xcafebabeu64;
            for _ in 0..k {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                seeds.push(state);
            }
            MinHasher { seeds }
        }

        /// Calcula la firma MinHash (longitud `k`) de un conjunto de *hashes*
        /// de tokens pre‑calculados (ya representados como `u64`).
        ///
        /// Para cada función hash (semilla) se obtiene el mínimo valor entre
        /// todos los elementos after aplicar la permutación hash.
        ///
        /// Complejidad temporal: O(k * n), donde n es el tamaño del conjunto.
        /// Complejidad espacial: O(k) para la firma.
        pub fn signature(&self, token_hashes: &[u64]) -> Vec<u64> {
            self.seeds
                .iter()
                .map(|&seed| {
                    token_hashes
                        .iter()
                        .map(|&h| hash_with_seed(h, seed))
                        .min()
                        .unwrap_or(u64::MAX)
                })
                .collect()
        }

        /// Estima la similitud de Jaccard entre dos firmas MinHash.
        ///
        /// La estimación es la proporción de posiciones coincidentes:
        /// J ≈ (1/k) * Σ [sig1[i] == sig2[i]]
        ///
        /// El error relativo disminuye con O(1/√k).
        pub fn estimated_jaccard(&self, sig1: &[u64], sig2: &[u64]) -> f32 {
            assert_eq!(sig1.len(), sig2.len());
            let matches = sig1.iter().zip(sig2).filter(|(a, b)| a == b).count();
            matches as f32 / sig1.len() as f32
        }
    }

    /// Función hash rápida que combina un valor `u64` con una semilla,
    /// generando una nueva dispersión uniforme.
    ///
    /// Internamente utiliza AHash con claves de 128 bits derivadas de la semilla.
    fn hash_with_seed(value: u64, seed: u64) -> u64 {
        let hash_builder = RandomState::with_seeds(seed, seed.wrapping_mul(0x9E3779B97F4A7C15), 0, 0);
        hash_builder.hash_one(value)
    }
}

// ---------------------------------------------------------------------------
// Tipo seguro para puntuaciones de similitud
// ---------------------------------------------------------------------------
/// Representa una puntuación de similitud en el rango [0, 1].
///
/// Se garantiza que el valor interno siempre es válido gracias a la
/// validación en el constructor.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SimilarityScore(f32);

impl SimilarityScore {
    /// Crea un `SimilarityScore` tras validar que `score ∈ [0, 1]`.
    ///
    /// # Panics
    /// Si `score` está fuera del rango permitido.
    pub fn new(score: f32) -> Self {
        assert!((0.0..=1.0).contains(&score), "Score fuera de rango: {}", score);
        SimilarityScore(score)
    }

    /// Devuelve el valor en punto flotante.
    pub fn value(&self) -> f32 {
        self.0
    }
}

impl std::fmt::Display for SimilarityScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Trait genérico para indexación Jaccard
// ---------------------------------------------------------------------------
/// Representa un elemento que puede ser indexado mediante un conjunto de tokens.
///
/// `T` es el tipo de token; debe ser ordenable y *hasheable* para que
/// las operaciones de intersección y MinHash funcionen correctamente.
pub trait JaccardIndex<T: Ord + Hash> {
    /// Devuelve una referencia al conjunto de tokens **ordenado**.
    fn token_set(&self) -> &[T];
}

// Implementación para vectores de hashes `u64` (el caso más común).
impl JaccardIndex<u64> for Vec<u64> {
    fn token_set(&self) -> &[u64] {
        self.as_slice()
    }
}

// ---------------------------------------------------------------------------
// Motor de catálogo (estructura columnar y paralelización)
// ---------------------------------------------------------------------------
/// Catálogo de productos pre‑procesados con sus *hashes* de tokens.
///
/// Almacena los *hashes* de todos los productos en un único buffer contiguo
/// (`all_hashes`) para maximizar la localidad de caché. Los límites de
/// cada producto se guardan en el vector `offsets` (longitud N+1).
pub struct Catalog {
    /// Todos los *hashes* de tokens, concatenados y ordenados por producto.
    all_hashes: Vec<u64>,
    /// offsets[i] = inicio del producto i, offsets[i+1] = final (exclusivo).
    offsets: Vec<usize>,
    /// Nombres originales (para depuración/visualización).
    names: Vec<String>,
}

impl Catalog {
    /// Construye un catálogo a partir de una lista de nombres de producto.
    ///
    /// La tokenización y ordenación se realizan una sola vez para cada registro.
    pub fn from_product_names(products: Vec<String>) -> Self {
        let mut all_hashes = Vec::new();
        let mut offsets = Vec::with_capacity(products.len() + 1);
        offsets.push(0);

        for name in &products {
            let mut hashes = tokenizer::tokenize_and_hash(name);
            all_hashes.append(&mut hashes);
            offsets.push(all_hashes.len());
        }

        Catalog {
            all_hashes,
            offsets,
            names: products,
        }
    }

    /// Obtiene el *slice* de hashes del producto en la posición `idx`.
    #[inline]
    fn get_hashes(&self, idx: usize) -> &[u64] {
        let start = self.offsets[idx];
        let end = self.offsets[idx + 1];
        &self.all_hashes[start..end]
    }

    /// Realiza una búsqueda desordenada (bolsa de palabras) **exacta**
    /// de la `query` contra todo el catálogo, devolviendo los `top_k`
    /// productos más similares según el coeficiente de Jaccard.
    ///
    /// La búsqueda se paraleliza con Rayon para aprovechar múltiples núcleos.
    ///
    /// Retorna un vector de tuplas `(nombre, SimilarityScore)` ordenado
    /// de mayor a menor similitud.
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(String, SimilarityScore)> {
        let query_hashes = tokenizer::tokenize_and_hash(query);

        // Paralelizamos la evaluación de todos los productos
        let mut scored: Vec<(usize, f32)> = (0..self.offsets.len() - 1)
            .into_par_iter()
            .map(|idx| {
                let score = set_ops::jaccard(&query_hashes, self.get_hashes(idx));
                (idx, score)
            })
            .collect();

        // Ordenamos por score descendente y nos quedamos con los top_k
        scored.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(top_k);

        scored
            .into_iter()
            .map(|(idx, score)| {
                (
                    self.names[idx].clone(),
                    SimilarityScore::new(score),
                )
            })
            .collect()
    }

    /// Encuentra pares de productos con similitud >= `threshold`.
    ///
    /// Útil para detección de duplicados en el catálogo.
    /// Advertencia: complejidad O(N²) set‑wise; para catálogos grandes
    /// considérese un pre‑filtro con MinHash.
    pub fn detect_duplicates(&self, threshold: f32) -> Vec<(usize, usize, SimilarityScore)> {
        let n = self.offsets.len() - 1;
        let mut result = Vec::new();
        for i in 0..n {
            let hashes_i = self.get_hashes(i);
            for j in (i + 1)..n {
                let score = set_ops::jaccard(hashes_i, self.get_hashes(j));
                if score >= threshold {
                    result.push((i, j, SimilarityScore::new(score)));
                }
            }
        }
        result
    }

    /// Obtiene los `top_k` productos más similares al producto en posición `idx`,
    /// excluyéndose a sí mismo. Implementa un **motor de recomendación** básico.
    pub fn recommend(&self, idx: usize, top_k: usize) -> Vec<(String, SimilarityScore)> {
        let target_hashes = self.get_hashes(idx);
        let n = self.offsets.len() - 1;

        let mut scored: Vec<(usize, f32)> = (0..n)
            .into_par_iter()
            .filter(|&i| i != idx)
            .map(|i| {
                let score = set_ops::jaccard(target_hashes, self.get_hashes(i));
                (i, score)
            })
            .collect();

        scored.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(top_k);

        scored
            .into_iter()
            .map(|(i, score)| {
                (
                    self.names[i].clone(),
                    SimilarityScore::new(score),
                )
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Ejemplo de uso y benchmark básico
// ---------------------------------------------------------------------------
fn main() {
    // Datos de ejemplo
    let products = vec![
        "Tornillo Hexagonal".to_string(),
        "Tuerca Hexagonal".to_string(),
        "Arandela Plana".to_string(),
        "Tornillo de Cabeza Plana".to_string(),
        "Hexagonal Tornillo".to_string(), // duplicado semántico para las pruebas
    ];

    // Construcción del catálogo
    let catalog = Catalog::from_product_names(products);

    // 1. Búsqueda desordenada (Disorder-Resilient Search)
    let query = "Hexagonal Tornillo";
    println!("🔍 Búsqueda: \"{}\"", query);
    let results = catalog.search(query, 3);
    for (name, score) in &results {
        println!("  {} → {}", name, score);
    }
    // Debe devolver "Tornillo Hexagonal" (y "Hexagonal Tornillo") con score 1.0

    // 2. Detección de duplicados
    println!("\n🧹 Duplicados (≥0.9):");
    let dups = catalog.detect_duplicates(0.9);
    for (i, j, score) in &dups {
        println!("  {} <-> {} → {}", catalog.names[*i], catalog.names[*j], score);
    }

    // 3. Motor de recomendación (producto similar a "Tornillo Hexagonal")
    let target_idx = 0;
    println!("\n⭐ Recomendaciones para \"{}\":", catalog.names[target_idx]);
    let recs = catalog.recommend(target_idx, 3);
    for (name, score) in &recs {
        println!("  {} → {}", name, score);
    }

    // 4. Benchmark simple de consultas por segundo (QPS)
    let queries = vec![
        "plana arandela",
        "tornillo cabeza",
        "tuerca",
        "hexagonal",
    ];
    let iterations = 50_000;
    let start = Instant::now();
    for _ in 0..iterations {
        for q in &queries {
            let _ = catalog.search(q, 3);
        }
    }
    let elapsed = start.elapsed();
    let total_queries = iterations * queries.len();
    let qps = total_queries as f64 / elapsed.as_secs_f64();
    println!("\n🚀 Benchmark: {} consultas en {:?} ({:.0} QPS)", total_queries, elapsed, qps);
}

// Asegurarse de que el crate `lazy_static` está disponible
// (en Cargo.toml agregar: lazy_static = "1.4")
// Si no se desea esa dependencia, se puede construir el set de stop‑words
// de forma manual. Se incluye por brevedad.
//
// También son necesarios: ahash, rayon, unicode-normalization.