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

        let query_phonetic = if let ffi::AlgoritmoType::Phonetic = algoritmo {
            let encoder = phonetic_index::phonetic_core::DoubleMetaphone::default();
            use phonetic_index::phonetic_core::PhoneticEncoder;
            Some(encoder.encode_primary(query))
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
                        // Comparamos contra id_codigo si tienen la misma longitud
                        if q_bytes.len() == id_bytes.len() && !q_bytes.is_empty() {
                            hamming_hpc_engine::simd::hamming_distance_u8(q_bytes, id_bytes)
                                .map(|d| 1.0 - (d as f64 / q_bytes.len() as f64))
                                .unwrap_or(0.0)
                        } else {
                            // Fallback: si el usuario busca algo que no coincide en longitud, 
                            // podríamos probar contra una parte o retornar 0.
                            0.0
                        }
                    },
                    ffi::AlgoritmoType::DamerauLevenshtein => {
                        use fuzzy_search_engine::metric::{DistanceMetric, DamerauLevenshtein};
                        use fuzzy_search_engine::prelude::to_grapheme_clusters;
                        let metric = DamerauLevenshtein::default();
                        let a = to_grapheme_clusters(query);
                        let b = to_grapheme_clusters(&item.descripcion_articulo);
                        let dist = metric.distance(&a, &b);
                        fuzzy_search_engine::Similarity::from_distance(dist, a.len().max(b.len())).raw()
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
                    ffi::AlgoritmoType::Phonetic => {
                        let encoder = phonetic_index::phonetic_core::DoubleMetaphone::default();
                        use phonetic_index::phonetic_core::PhoneticEncoder;
                        let item_phonetic = encoder.encode_primary(&item.descripcion_articulo);
                        if let Some(ref q_phonetic) = query_phonetic {
                            if q_phonetic == &item_phonetic { 1.0 } else { 0.0 }
                        } else {
                            0.0
                        }
                    },
                    ffi::AlgoritmoType::JaroWinkler => {
                        jaro_winkler_engine::jaro_winkler(query, &item.descripcion_articulo)
                    },
                    ffi::AlgoritmoType::Cosine => {
                        // El score de Coseno se maneja de forma especial mediante HNSW
                        // En este loop lineal, podríamos implementarlo, pero usaremos el HNSW si está disponible.
                        0.0
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

        // Caso especial: Búsqueda Semántica (Cosine) via HNSW
        if let ffi::AlgoritmoType::Cosine = algoritmo {
            if let (Some(ref cat), Some(ref hnsw)) = (&self.semantic_catalog, &self.hnsw) {
                let backend = semantic_engine::embedding_bridge::MockEmbeddingBackend;
                use semantic_engine::embedding_bridge::EmbeddingBackend;
                let q_emb = backend.encode(query);
                
                let hnsw_results = hnsw.search(cat, q_emb.as_slice(), 50, 200);
                return hnsw_results.into_iter().map(|res| ffi::SearchResult {
                    id: res.label,
                    nombre: String::new(), // HNSW no guarda el nombre original en el mock, solo el ID
                    score: res.score as f64,
                }).collect();
            }
        }

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
    })
}
