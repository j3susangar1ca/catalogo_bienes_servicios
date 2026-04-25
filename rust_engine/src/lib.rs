use std::sync::Arc;
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

#[cxx::bridge]
mod ffi {
    enum AlgoritmoType {
        Hamming,
        SorensenDice,
        Phonetic,
        DamerauLevenshtein,
        Jaccard,
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
    // Aquí podrías añadir índices pre-calculados (ej. BKTree)
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
                true
            }
            Err(e) => {
                eprintln!("Error al abrir el CSV {}: {}", ruta_csv, e);
                false
            }
        }
    }

    pub fn buscar(&self, query: &str, algoritmo: ffi::AlgoritmoType) -> Vec<ffi::SearchResult> {
        let query_str = query.to_lowercase();
        
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
                        // Comparamos contra id_codigo
                        // Nota: hamming_hpc_engine requiere nightly por portable_simd
                        hamming_hpc_engine::simd::hamming_distance_u8(query.as_bytes(), item.id_codigo.as_bytes())
                            .map(|d| 1.0 - (d as f64 / 10.0))
                            .unwrap_or(0.0)
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
    Box::new(SearchMaster { catalogo: Vec::new() })
}
