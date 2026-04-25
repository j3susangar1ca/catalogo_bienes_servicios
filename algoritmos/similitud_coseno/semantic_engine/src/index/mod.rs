// ============================================================
//  MODULE: index
//  Búsqueda exacta (lineal + Rayon) y aproximada (HNSW).
// ============================================================

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;
use ordered_float::OrderedFloat;
use rayon::prelude::*;

use crate::vector_math::{dot_simd, normalize_inplace, DIM};

// ─────────────────────────────────────────────────────────────
//  CATÁLOGO CENTRAL — Structure of Arrays (SoA)
// ─────────────────────────────────────────────────────────────
/// Almacena N vectores de forma plana y contigua.
/// Layout: [ v0[0..DIM] | v1[0..DIM] | … | vN[0..DIM] ]
/// Esto maximiza la localidad de caché durante el barrido lineal.
pub struct VectorCatalog {
    /// Buffer plano, tamaño = N × DIM (f32, alineado conceptualmente).
    pub data: Vec<f32>,
    /// Etiquetas / descripciones de cada registro.
    pub labels: Vec<String>,
    /// Número de registros.
    pub n: usize,
    /// Dimensión de cada vector.
    pub dim: usize,
    /// ¿Los vectores ya están normalizados?
    pub normalized: bool,
}

impl VectorCatalog {
    // ----------------------------------------------------------
    //  CONSTRUCCIÓN
    // ----------------------------------------------------------
    pub fn new(dim: usize) -> Self {
        Self {
            data: Vec::new(),
            labels: Vec::new(),
            n: 0,
            dim,
            normalized: false,
        }
    }

    /// Agrega un vector con su etiqueta.  
    /// `vec` debe tener exactamente `self.dim` elementos.
    pub fn push(&mut self, vec: &[f32], label: impl Into<String>) {
        assert_eq!(vec.len(), self.dim, "push: dimensión incorrecta");
        self.data.extend_from_slice(vec);
        self.labels.push(label.into());
        self.n += 1;
    }

    /// Retorna el slice del i-ésimo vector — **zero-copy**.
    #[inline(always)]
    pub fn get(&self, i: usize) -> &[f32] {
        let start = i * self.dim;
        &self.data[start..start + self.dim]
    }

    /// Retorna el slice mutable del i-ésimo vector.
    #[inline(always)]
    fn get_mut(&mut self, i: usize) -> &mut [f32] {
        let start = i * self.dim;
        let end   = start + self.dim;
        &mut self.data[start..end]
    }

    // ----------------------------------------------------------
    //  PRE-NORMALIZACIÓN (acelera la búsqueda en runtime)
    // ----------------------------------------------------------
    /// Normaliza todos los vectores del catálogo in-place.
    /// Después de esto: cosine(A,B) ≡ dot(A,B) → búsqueda más rápida.
    pub fn normalize_all(&mut self) {
        let dim = self.dim;
        let n   = self.n;
        // Paralelizamos con Rayon: cada thread normaliza su chunk.
        self.data
            .par_chunks_mut(dim)
            .take(n)
            .for_each(|chunk| normalize_inplace(chunk));
        self.normalized = true;
    }
}

// ─────────────────────────────────────────────────────────────
//  RESULTADO DE BÚSQUEDA
// ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub index: usize,
    pub label: String,
    pub score: f32,
}

// ─────────────────────────────────────────────────────────────
//  BÚSQUEDA LINEAL EXACTA (paralelizada con Rayon)
// ─────────────────────────────────────────────────────────────
/// Compara la consulta contra todos los N registros del catálogo.
/// Complejidad: O(N × D).  
/// Con Rayon, el trabajo se divide en `num_cpus` chunks automáticamente.
pub fn linear_search(
    catalog: &VectorCatalog,
    query: &[f32],
    top_k: usize,
) -> Vec<SearchResult> {
    assert_eq!(query.len(), catalog.dim, "query: dimensión incorrecta");

    // Cada chunk de Rayon devuelve sus K mejores; luego mergeamos.
    let scores: Vec<(usize, f32)> = (0..catalog.n)
        .into_par_iter()
        .map(|i| {
            let vec = catalog.get(i);
            // Si el catálogo está normalizado: cosine ≡ dot (más rápido).
            let score = if catalog.normalized {
                dot_simd(query, vec)
            } else {
                // similitud coseno completa
                let dot   = dot_simd(query, vec);
                let nq    = dot_simd(query, query).sqrt();
                let nv    = dot_simd(vec, vec).sqrt();
                if nq < 1e-10 || nv < 1e-10 { 0.0 } else { dot / (nq * nv) }
            };
            (i, score)
        })
        .collect();

    // Selección de top-K usando un min-heap de tamaño K.
    let mut heap: BinaryHeap<Reverse<(OrderedFloat<f32>, usize)>> =
        BinaryHeap::with_capacity(top_k + 1);

    for (i, s) in &scores {
        heap.push(Reverse((OrderedFloat(*s), *i)));
        if heap.len() > top_k {
            heap.pop(); // descarta el peor
        }
    }

    // Resultado en orden descendente de score.
    let mut results: Vec<SearchResult> = heap
        .into_iter()
        .map(|Reverse((score, i))| SearchResult {
            index: i,
            label: catalog.labels[i].clone(),
            score: score.into_inner(),
        })
        .collect();
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results
}

// ─────────────────────────────────────────────────────────────
//  HNSW — Hierarchical Navigable Small World
//  Búsqueda Aproximada de Vecinos (ANNS), complejidad O(log N)
// ─────────────────────────────────────────────────────────────

/// Nodo en el grafo HNSW.
struct HnswNode {
    /// Vecinos en cada capa.  `neighbors[layer][i]` = índice del vecino.
    neighbors: Vec<Vec<usize>>,
}

/// Índice HNSW sobre un `VectorCatalog`.
pub struct HnswIndex {
    nodes: Vec<HnswNode>,
    /// Punto de entrada (nodo con la capa más alta).
    entry_point: Option<usize>,
    /// Número de capas del nodo de entrada.
    max_layer: usize,
    // ── Hiperparámetros ──────────────────────────────────────
    /// M: máximo de conexiones bidireccionales por nodo.
    m: usize,
    /// M₀: conexiones en la capa 0 (suele ser 2×M).
    m0: usize,
    /// ef_construction: tamaño de la lista de candidatos durante la inserción.
    ef_construction: usize,
    /// Multiplicador para la distribución de capas (≈ 1/ln(M)).
    ml: f64,
}

impl HnswIndex {
    // ----------------------------------------------------------
    //  CONFIGURACIÓN
    // ----------------------------------------------------------
    /// Crea un índice vacío.
    /// - `m`: 16 es un buen default para 768-dim con 64k registros.
    /// - `ef_construction`: 200 da buen recall; reducir para mayor velocidad.
    pub fn new(m: usize, ef_construction: usize) -> Self {
        Self {
            nodes: Vec::new(),
            entry_point: None,
            max_layer: 0,
            m,
            m0: m * 2,
            ef_construction,
            ml: 1.0 / (m as f64).ln(),
        }
    }

    // ----------------------------------------------------------
    //  INSERCIÓN
    // ----------------------------------------------------------
    /// Inserta el i-ésimo vector del catálogo en el índice.
    pub fn insert(&mut self, catalog: &VectorCatalog, id: usize) {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Determina la capa máxima de este nodo (distribución exponencial).
        let node_layer = {
            let r: f64 = rng.gen::<f64>();
            (-r.ln() * self.ml).floor() as usize
        };

        // Inicializa el nodo con listas de vecinos vacías.
        let mut node = HnswNode {
            neighbors: vec![Vec::new(); node_layer + 1],
        };

        // ── Caso: primer nodo ──
        if self.entry_point.is_none() {
            self.nodes.push(node);
            self.entry_point = Some(id);
            self.max_layer   = node_layer;
            return;
        }

        let ep = self.entry_point.unwrap();
        let ep_layer = self.max_layer;

        let q = catalog.get(id);

        // ── Fase 1: greedy desde la capa más alta hasta node_layer+1 ──
        let mut curr_ep = ep;
        for layer in (node_layer + 1..=ep_layer).rev() {
            let candidates = self.search_layer(catalog, q, curr_ep, 1, layer);
            if let Some(&best) = candidates.iter().min_by(|&&a, &&b| {
                let da = dist(catalog.get(a), q);
                let db = dist(catalog.get(b), q);
                da.partial_cmp(&db).unwrap()
            }) {
                curr_ep = best;
            }
        }

        // ── Fase 2: inserción en capas [0..node_layer] ──
        for layer in (0..=node_layer.min(ep_layer)).rev() {
            let m_layer = if layer == 0 { self.m0 } else { self.m };
            let mut candidates =
                self.search_layer(catalog, q, curr_ep, self.ef_construction, layer);

            // Selecciona los M vecinos más cercanos para este nodo.
            candidates.sort_by(|&a, &b| {
                dist(catalog.get(a), q)
                    .partial_cmp(&dist(catalog.get(b), q))
                    .unwrap()
            });
            let neighbors: Vec<usize> =
                candidates.iter().cloned().take(m_layer).collect();

            node.neighbors[layer] = neighbors.clone();

            // Actualiza los vecinos en el grafo (bidireccional).
            for &nb in &neighbors {
                if nb < self.nodes.len() {
                    if layer < self.nodes[nb].neighbors.len() {
                        self.nodes[nb].neighbors[layer].push(id);
                        // Poda si excede M.
                        let max_m = if layer == 0 { self.m0 } else { self.m };
                        if self.nodes[nb].neighbors[layer].len() > max_m {
                            let nb_vec = catalog.get(nb);
                            self.nodes[nb].neighbors[layer].sort_by(|&a, &b| {
                                let da = if a < catalog.n { dist(catalog.get(a), nb_vec) } else { f32::MAX };
                                let db = if b < catalog.n { dist(catalog.get(b), nb_vec) } else { f32::MAX };
                                da.partial_cmp(&db).unwrap()
                            });
                            self.nodes[nb].neighbors[layer].truncate(max_m);
                        }
                    }
                }
            }

            if let Some(&best) = candidates.first() {
                curr_ep = best;
            }
        }

        // Actualiza el entry point si este nodo tiene una capa más alta.
        if node_layer > self.max_layer {
            self.max_layer = node_layer;
            self.entry_point = Some(id);
        }

        // Registra el nodo (el índice en `nodes` debe coincidir con `id`).
        // Para catálogos que insertan en orden 0..N esto es correcto.
        if id >= self.nodes.len() {
            self.nodes.resize_with(id + 1, || HnswNode { neighbors: Vec::new() });
        }
        self.nodes[id] = node;
    }

    // ----------------------------------------------------------
    //  BÚSQUEDA EN UNA SOLA CAPA (algoritmo greedy expandido)
    // ----------------------------------------------------------
    fn search_layer(
        &self,
        catalog: &VectorCatalog,
        query: &[f32],
        ep: usize,
        ef: usize,
        layer: usize,
    ) -> Vec<usize> {
        let mut visited: HashSet<usize> = HashSet::new();
        // Min-heap de candidatos (más cercanos primero).
        let mut candidates: BinaryHeap<Reverse<(OrderedFloat<f32>, usize)>> =
            BinaryHeap::new();
        // Max-heap de resultados (para mantener los `ef` mejores).
        let mut result: BinaryHeap<(OrderedFloat<f32>, usize)> = BinaryHeap::new();

        let d_ep = dist(catalog.get(ep), query);
        candidates.push(Reverse((OrderedFloat(d_ep), ep)));
        result.push((OrderedFloat(d_ep), ep));
        visited.insert(ep);

        while let Some(Reverse((OrderedFloat(d_curr), curr))) = candidates.pop() {
            // Si el más cercano candidato es peor que el peor resultado: parar.
            if let Some(&(OrderedFloat(d_worst), _)) = result.peek() {
                if d_curr > d_worst && result.len() >= ef {
                    break;
                }
            }

            // Expande vecinos del nodo actual en esta capa.
            if curr < self.nodes.len() && layer < self.nodes[curr].neighbors.len() {
                for &nb in &self.nodes[curr].neighbors[layer] {
                    if !visited.contains(&nb) && nb < catalog.n {
                        visited.insert(nb);
                        let d_nb = dist(catalog.get(nb), query);
                        if let Some(&(OrderedFloat(d_worst), _)) = result.peek() {
                            if d_nb < d_worst || result.len() < ef {
                                candidates.push(Reverse((OrderedFloat(d_nb), nb)));
                                result.push((OrderedFloat(d_nb), nb));
                                if result.len() > ef {
                                    result.pop(); // descarta el más lejano
                                }
                            }
                        } else {
                            candidates.push(Reverse((OrderedFloat(d_nb), nb)));
                            result.push((OrderedFloat(d_nb), nb));
                        }
                    }
                }
            }
        }

        result.into_iter().map(|(_, id)| id).collect()
    }

    // ----------------------------------------------------------
    //  BÚSQUEDA APROXIMADA (K vecinos más cercanos)
    // ----------------------------------------------------------
    /// Retorna los `top_k` registros más similares a `query`.
    /// Complejidad aprox. O(log N) tras construir el índice.
    pub fn search(
        &self,
        catalog: &VectorCatalog,
        query: &[f32],
        top_k: usize,
        ef_search: usize,
    ) -> Vec<SearchResult> {
        let ep = match self.entry_point {
            Some(ep) => ep,
            None      => return Vec::new(),
        };

        // Descenso greedy desde la capa más alta hasta la capa 1.
        let mut curr_ep = ep;
        for layer in (1..=self.max_layer).rev() {
            let candidates =
                self.search_layer(catalog, query, curr_ep, 1, layer);
            if let Some(&best) = candidates.iter().min_by(|&&a, &&b| {
                dist(catalog.get(a), query)
                    .partial_cmp(&dist(catalog.get(b), query))
                    .unwrap()
            }) {
                curr_ep = best;
            }
        }

        // Búsqueda en capa 0 con ef_search candidatos.
        let mut candidates =
            self.search_layer(catalog, query, curr_ep, ef_search.max(top_k), 0);

        // Ordena por similitud descendente (usamos distancia inversa).
        candidates.sort_by(|&a, &b| {
            let da = dist(catalog.get(a), query);
            let db = dist(catalog.get(b), query);
            da.partial_cmp(&db).unwrap()
        });

        candidates
            .into_iter()
            .take(top_k)
            .map(|i| {
                let d_sq = dist_squared(catalog.get(i), query);
                // Convertimos distancia euclídea cuadrada a similitud coseno aproximada
                // (válido cuando los vectores están normalizados: cos ≈ 1 - ||a-b||²/2).
                let score = 1.0 - d_sq / 2.0;
                SearchResult {
                    index: i,
                    label: catalog.labels[i].clone(),
                    score,
                }
            })
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────
//  UTILIDAD: distancia euclídea cuadrada (sin sqrt para comparar)
// ─────────────────────────────────────────────────────────────
/// Distancia euclídea al cuadrado entre dos vectores.
/// No aplica sqrt() porque para comparaciones en HNSW solo se necesita monotonicidad.
/// Esto ahorra ~10-20 ciclos por llamada, crítico en búsquedas de millones de vectores.
#[inline(always)]
fn dist_squared(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum::<f32>()
}

// Alias para compatibilidad con código existente que llama a `dist`
#[inline(always)]
fn dist(a: &[f32], b: &[f32]) -> f32 {
    dist_squared(a, b)
}
