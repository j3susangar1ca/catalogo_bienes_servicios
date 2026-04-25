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
        
        // Uso de RAYON para paralelismo en tus 8 núcleos
        let mut resultados: Vec<ffi::SearchResult> = self.catalogo.par_iter()
            .filter(|r| r.activo == 1) // Solo productos activos
            .map(|item| {
                let score = match algoritmo {
                    ffi::AlgoritmoType::Hamming => {
                        // Comparamos contra id_codigo
                        1.0 - (hamming_hpc_engine::calculate_distance(query, &item.id_codigo) as f64 / 10.0)
                    },
                    ffi::AlgoritmoType::DamerauLevenshtein => {
                        // Tu algoritmo True Damerau-Levenshtein
                        fuzzy_search_engine::compare(query, &item.descripcion_articulo)
                    },
                    ffi::AlgoritmoType::SorensenDice => {
                        sorensen_dice_engine::calculate(query, &item.descripcion_larga_art)
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
    Box::new(SearchMaster { catalogo: Vec::new() })
}
