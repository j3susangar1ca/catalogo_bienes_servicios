use rayon::prelude::*;
use csv::ReaderBuilder;
use serde::Deserialize;

// Estructura para mapear el CSV
#[derive(Debug, Deserialize, Clone)]
struct Record {
    id_codigo: String,
    descripcion_articulo: String,
    descripcion_larga_art: String,
    unidad_medida: String,
    ultimo_precio: String,
    activo: i32,
}

#[cxx::bridge(namespace = "ffi")]
mod ffi {
    enum AlgoritmoType {
        Hamming,
        SorensenDice,
        Phonetic,
        DamerauLevenshtein,
        Jaccard,
        JaroWinkler,
        Cosine,
    }

    struct SearchResult {
        pub id: String,
        pub nombre: String,
        pub score: f64,
    }

    extern "Rust" {
        type SearchMaster;
        fn new_search_master() -> Box<SearchMaster>;
        fn cargar_catalogo(self: &mut SearchMaster, ruta_csv: &str) -> bool;
        fn buscar(self: &SearchMaster, query: &str, algoritmo: AlgoritmoType) -> Vec<SearchResult>;
    }
}

pub struct SearchMaster {
    catalogo: Vec<Record>,
    semantic_catalog: Option<semantic_engine::index::VectorCatalog>,
    hnsw: Option<semantic_engine::index::HnswIndex>,
    // Índices adicionales para acelerar búsquedas
    phonetic_index: Option<phonetic_index::index::PhoneticIndex>,
    bktree_damerau: Option<fuzzy_search_engine::index::BKTree<fuzzy_search_engine::metric::DamerauLevenshtein, String>>,
}

impl SearchMaster {
    pub fn cargar_catalogo(&mut self, ruta_csv: &str) -> bool {
        let rdr_res = ReaderBuilder::new()
            .has_headers(true)
            .from_path(ruta_csv);

        match rdr_res {
            Ok(mut rdr) => {
                // Carga masiva en RAM
                self.catalogo = rdr.deserialize().filter_map(|res: Result<Record, _>| res.ok()).collect();
                println!("Catálogo cargado: {} registros", self.catalogo.len());
                
                // Construcción de índice semántico (HNSW)
                let mut semantic_cat = semantic_engine::index::VectorCatalog::new(semantic_engine::vector_math::DIM);
                let backend = semantic_engine::embedding_bridge::MockEmbeddingBackend;
                use semantic_engine::embedding_bridge::EmbeddingBackend;

                for record in &self.catalogo {
                    let emb = backend.encode(&record.descripcion_articulo);
                    semantic_cat.push(emb.as_slice(), &record.id_codigo);
                }
                
                let mut hnsw = semantic_engine::index::HnswIndex::new(16, 200);
                for i in 0..semantic_cat.n {
                    hnsw.insert(&semantic_cat, i);
                }
                
                self.semantic_catalog = Some(semantic_cat);
                self.hnsw = Some(hnsw);
                
                // Construcción de índice fonético para búsquedas O(1)
                let phonetic_catalog: Vec<(phonetic_index::index::RecordId, String)> = self.catalogo
                    .iter()
                    .enumerate()
                    .map(|(i, r)| (i as phonetic_index::index::RecordId, r.descripcion_articulo.clone()))
                    .collect();
                self.phonetic_index = Some(phonetic_index::index::PhoneticIndex::build(&phonetic_catalog));
                
                // Construcción de BK-Tree para Damerau-Levenshtein
                let mut bktree: fuzzy_search_engine::index::BKTree<fuzzy_search_engine::metric::DamerauLevenshtein, String> = 
                    fuzzy_search_engine::index::BKTree::new();
                for (i, record) in self.catalogo.iter().enumerate() {
                    bktree.insert(record.descripcion_articulo.clone(), record.id_codigo.clone());
                }
                self.bktree_damerau = Some(bktree);
                
                true
            }
            Err(e) => {
                eprintln!("Error al abrir el CSV {}: {}", ruta_csv, e);
                false
            }
        }
    }

    pub fn buscar(&self, query: &str, algoritmo: ffi::AlgoritmoType) -> Vec<ffi::SearchResult> {
        let _query_str = query.to_lowercase();

        // --- 1. BÚSQUEDAS INDEXADAS (O(log N) o O(1)) ---

        // Caso: Damerau-Levenshtein via BK-Tree
        if let ffi::AlgoritmoType::DamerauLevenshtein = algoritmo {
            if let Some(ref bktree) = self.bktree_damerau {
                use fuzzy_search_engine::types::Similarity;
                let results = bktree.search(query, Similarity::from_threshold(0.85));
                return results.into_iter().map(|id| {
                    let nombre = self.catalogo.iter()
                        .find(|r| r.id_codigo == id)
                        .map(|r| r.descripcion_articulo.clone())
                        .unwrap_or_default();
                    ffi::SearchResult { id, nombre, score: 1.0 }
                }).take(50).collect();
            }
        }

        // Caso: Fonético via PhoneticIndex
        if let ffi::AlgoritmoType::Phonetic = algoritmo {
            if let Some(ref ph_idx) = self.phonetic_index {
                let results = ph_idx.search(query);
                if !results.is_empty() {
                    return results.into_iter().filter_map(|idx| {
                        self.catalogo.get(idx as usize).map(|r| ffi::SearchResult {
                            id: r.id_codigo.clone(),
                            nombre: r.descripcion_articulo.clone(),
                            score: 1.0,
                        })
                    }).take(50).collect();
                }
                let fuzzy_results = ph_idx.fuzzy_search(query, 2);
                return fuzzy_results.into_iter().filter_map(|(idx, dist)| {
                    self.catalogo.get(idx as usize).map(|r| ffi::SearchResult {
                        id: r.id_codigo.clone(),
                        nombre: r.descripcion_articulo.clone(),
                        score: 1.0 - (dist as f64 * 0.3),
                    })
                }).take(50).collect();
            }
        }

        // Caso especial: Búsqueda Semántica (Cosine) via HNSW
        if let ffi::AlgoritmoType::Cosine = algoritmo {
            if let (Some(ref cat), Some(ref hnsw)) = (&self.semantic_catalog, &self.hnsw) {
                let backend = semantic_engine::embedding_bridge::MockEmbeddingBackend;
                use semantic_engine::embedding_bridge::EmbeddingBackend;
                let q_emb = backend.encode(query);
                
                let hnsw_results = hnsw.search(cat, q_emb.as_slice(), 50, 200);
                return hnsw_results.into_iter().map(|res| {
                    let nombre = self.catalogo.iter()
                        .find(|r| r.id_codigo == res.label)
                        .map(|r| r.descripcion_articulo.clone())
                        .unwrap_or_default();
                    ffi::SearchResult {
                        id: res.label,
                        nombre,
                        score: res.score as f64,
                    }
                }).collect();
            }
        }

        // --- 2. BÚSQUEDAS LINEALES (PARALELIZADAS CON RAYON) ---

        // Pre-procesamiento de query según el algoritmo para evitar trabajo redundante en el loop
        let query_shingles = if let ffi::AlgoritmoType::SorensenDice = algoritmo {
            Some(sorensen_dice_engine::shingler::generate_shingles(query))
        } else {
            None
        };

        let query_tokens_jaccard = if let ffi::AlgoritmoType::Jaccard = algoritmo {
            Some(jaccard_engine::tokenizer::tokenize_and_hash(query))
        } else {
            None
        };

        // Uso de RAYON para paralelismo en tus 8 núcleos
        let mut resultados: Vec<ffi::SearchResult> = self.catalogo.par_iter()
            .filter(|r| r.activo == 1) // Solo productos activos
            .map(|item| {
                let score = match algoritmo {
                    ffi::AlgoritmoType::Hamming => {
                        let q_bytes = query.as_bytes();
                        let id_bytes = item.id_codigo.as_bytes();
                        if q_bytes.len() == id_bytes.len() && !q_bytes.is_empty() {
                            hamming_hpc_engine::simd::hamming_distance_u8(q_bytes, id_bytes)
                                .map(|d| 1.0 - (d as f64 / q_bytes.len() as f64))
                                .unwrap_or(0.0)
                        } else {
                            0.0
                        }
                    },
                    ffi::AlgoritmoType::SorensenDice => {
                        let item_shingles = sorensen_dice_engine::shingler::generate_shingles(&item.descripcion_larga_art);
                        if let Some(ref q_shingles) = query_shingles {
                            sorensen_dice_engine::scoring::dice_similarity(q_shingles, &item_shingles, 0.0).unwrap_or(0.0)
                        } else {
                            0.0
                        }
                    },
                    ffi::AlgoritmoType::Jaccard => {
                        let item_tokens = jaccard_engine::tokenizer::tokenize_and_hash(&item.descripcion_articulo);
                        if let Some(ref q_tokens) = query_tokens_jaccard {
                            jaccard_engine::set_ops::jaccard(q_tokens, &item_tokens) as f64
                        } else {
                            0.0
                        }
                    },
                    ffi::AlgoritmoType::JaroWinkler => {
                        use jaro_winkler_engine::JaroWinklerMatcher;
                        let matcher = JaroWinklerMatcher::new(0.1, 4).expect("Invalid Jaro-Winkler config");
                        matcher.similarity(query, &item.descripcion_articulo).unwrap_or(0.0)
                    },
                    _ => 0.0,
                };

                ffi::SearchResult {
                    id: item.id_codigo.clone(),
                    nombre: item.descripcion_articulo.clone(),
                    score,
                }
            })
            .filter(|res| res.score > 0.1) // Umbral mínimo de relevancia
            .collect();

        // Ordenar por score de mayor a menor (Elite Ranking)
        resultados.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Retornar los 50 mejores para no saturar la UI
        resultados.into_iter().take(50).collect()
    }
}

pub fn new_search_master() -> Box<SearchMaster> {
    Box::new(SearchMaster { 
        catalogo: Vec::new(),
        semantic_catalog: None,
        hnsw: None,
        phonetic_index: None,
        bktree_damerau: None,
    })
}
