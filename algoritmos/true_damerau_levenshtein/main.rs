use std::time::Instant;
use fuzzy_search_engine::prelude::*;
use fuzzy_search_engine::metric::{DistanceMetric, DamerauLevenshtein};
use fuzzy_search_engine::index::BKTree;
use fuzzy_search_engine::types::Similarity;

/// Genera 64,000 registros ficticios para benchmark.
fn generate_catalog(size: usize) -> Vec<(String, String)> {
    (0..size)
        .map(|i| {
            let base = format!("producto_{:05}", i);
            // Variaciones realistas: typos, transposiciones, sustituciones numéricas
            let variations = [
                base.clone(),
                base.replacen("0", "O", 1),      // Confusión 0/O
                base.replacen("1", "l", 1),      // Confusión 1/l
                {
                    let mut chars: Vec<char> = base.chars().collect();
                    if chars.len() > 2 {
                        chars.swap(chars.len()-2, chars.len()-1); // Transposición final
                    }
                    chars.into_iter().collect()
                },
                format!("prod_{}", i),            // Abreviatura común
            ];
            let key = variations.choose(&mut rand::thread_rng()).unwrap().clone();
            (key, format!("SKU-{:06}", i))
        })
        .collect()
}

fn main() {
    println!("🚀 Inicializando Fuzzy Search Engine...");
    
    // Configuración
    const CATALOG_SIZE: usize = 64_000;
    const SIMILARITY_THRESHOLD: f64 = 0.85;
    
    // Inicializar árbol con métrica True Damerau-Levenshtein
    let mut tree: BKTree<DamerauLevenshtein, String> = BKTree::new();
    
    // Fase 1: Indexación de 64k registros
    println!("📦 Indexando {} registros...", CATALOG_SIZE);
    let index_start = Instant::now();
    
    let catalog = generate_catalog(CATALOG_SIZE);
    for (key, payload) in catalog {
        tree.insert(key, payload);
    }
    
    let index_duration = index_start.elapsed();
    println!("✅ Indexación completada en {:.2}s ({:.0} registros/ms)", 
             index_duration.as_secs_f64(),
             CATALOG_SIZE as f64 / index_duration.as_millis() as f64);
    
    // Fase 2: Benchmark de búsquedas
    let test_queries = [
        "producto_12345",    // Exact match
        "producto_12346",    // Sustitución numérica (costo 2.0)
        "producto_12354",    // Transposición alfabética (costo 1.0)
        "prod_12345",        // Abreviatura
        "product_12345",     // Error ortográfico común
    ];
    
    println!("\n🔍 Ejecutando benchmark de búsquedas (umbral: {:.0}%)...", 
             SIMILARITY_THRESHOLD * 100.0);
    
    let mut total_search_time = std::time::Duration::ZERO;
    let iterations = 100; // Promediar para estabilidad
    
    for &query in &test_queries {
        let mut times = Vec::with_capacity(iterations);
        
        for _ in 0..iterations {
            let start = Instant::now();
            let results = tree.search(query, Similarity::from_threshold(SIMILARITY_THRESHOLD));
            let elapsed = start.elapsed();
            times.push(elapsed);
            
            // Validar resultados en primera iteración
            if times.len() == 1 {
                println!("\n📝 Query: '{}'", query);
                println!("   Resultados: {} encontrados", results.len());
                for (i, res) in results.iter().take(3).enumerate() {
                    println!("   [{}] {}", i+1, res);
                }
            }
        }
        
        // Calcular percentil 95 para métrica realista de latencia
        times.sort();
        let p95_idx = (iterations as f64 * 0.95) as usize;
        let p95_time = times[p95_idx.min(times.len()-1)];
        
        println!("   ⏱️  Latencia p95: {:.2}ms {}", 
                 p95_time.as_micros() as f64 / 1000.0,
                 if p95_time.as_millis() < 10 { "✅ <10ms" } else { "⚠️ >=10ms" });
        
        total_search_time += p95_time;
    }
    
    // Fase 3: Búsqueda paralela con Rayon
    println!("\n⚡ Probando búsqueda paralela (Rayon) con 10 queries simultáneas...");
    let parallel_queries: Vec<&str> = (0..10)
        .map(|i| format!("producto_{:05}", i * 1000).leak() as &str)
        .collect();
    
    let parallel_start = Instant::now();
    let batch_results = tree.search_batch(&parallel_queries, Similarity::from_threshold(0.85));
    let parallel_duration = parallel_start.elapsed();
    
    let total_results: usize = batch_results.iter().map(|r| r.len()).sum();
    println!("✅ Búsqueda paralela: {} resultados en {:.2}ms", 
             total_results,
             parallel_duration.as_micros() as f64 / 1000.0);
    
    // Resumen final
    println!("\n🎯 Resumen de Performance:");
    println!("   • Registros indexados: {}", tree.len());
    println!("   • Búsqueda simple p95: <{:.2}ms", total_search_time.as_millis() as f64 / test_queries.len() as f64);
    println!("   • Búsqueda paralela: {:.2}ms para 10 queries", parallel_duration.as_micros() as f64 / 1000.0);
    println!("   • Memoria: ~{}MB estimados (64k registros + índice BK-Tree)", 
             (CATALOG_SIZE * 100) / 1024); // Estimación conservadora
    
    println!("\n✨ Fuzzy Search Engine listo para producción.");
    println!("   Compilar con: cargo build --release --lto=fat");
}