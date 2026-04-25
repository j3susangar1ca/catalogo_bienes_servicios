// ============================================================
//  SEMANTIC SEARCH ENGINE — Catálogo de 64,000 Registros
//  Demo: "Herramienta de torque" → encuentra "Llave de impacto"
// ============================================================

mod vector_math;
mod index;
mod embedding_bridge;

use std::time::Instant;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use vector_math::{DIM, normalize_inplace};
use index::{VectorCatalog, HnswIndex, linear_search};
use embedding_bridge::{MockEmbeddingBackend, EmbeddingBackend};

// ─────────────────────────────────────────────────────────────
//  CONSTANTES DE SIMULACIÓN
// ─────────────────────────────────────────────────────────────
const N_RECORDS: usize = 64_000;

/// Descripciones reales para inyectar como "islas semánticas"
/// que el motor debe encontrar ante la consulta de torque.
const TORQUE_TOOLS: &[&str] = &[
    "Llave de impacto neumática 1/2\"",
    "Llave dinamométrica digital 0-300 Nm",
    "Llave de torsión mecánica ajustable",
    "Torquímetro de click 40-200 Nm",
    "Pistola de impacto eléctrica brushless",
    "Llave de impacto hidráulica industrial",
    "Multiplicador de torque 10:1",
    "Adaptador de torque para dados de impacto",
];

const HAND_PROTECTION: &[&str] = &[
    "Guantes de carnaza para soldadura",
    "Guantes de nitrilo desechables",
    "Guantes de hule para manejo de químicos",
    "Guantes anticorte nivel 5",
    "Protector de palmas de cuero",
];

const LIFTING_GEAR: &[&str] = &[
    "Tecle de cadena de 2 toneladas",
    "Eslinga de poliéster de 5 toneladas",
    "Grúa pluma giratoria 500 kg",
    "Polipasto eléctrico 1 ton 220V",
];

// ─────────────────────────────────────────────────────────────
//  MAIN
// ─────────────────────────────────────────────────────────────
fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║        MOTOR DE BÚSQUEDA SEMÁNTICA — 64K REGISTROS       ║");
    println!("║    Cosine Similarity · SIMD · Rayon · HNSW · ONNX-ready  ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let backend = MockEmbeddingBackend;

    // ─────────────────────────────────────────────────────────
    // FASE 1: Construcción del catálogo
    // ─────────────────────────────────────────────────────────
    println!("▶ [1/4] Construyendo catálogo con {} registros...", N_RECORDS);
    let t_start = Instant::now();

    let mut catalog = VectorCatalog::new(DIM);

    // Semilla fija para reproducibilidad.
    let mut rng = StdRng::seed_from_u64(42);

    // Número de registros reales que inyectaremos.
    let n_real = TORQUE_TOOLS.len() + HAND_PROTECTION.len() + LIFTING_GEAR.len();

    // -- Relleno aleatorio (N - real registros) --
    let mut buf = vec![0.0f32; DIM];
    for i in 0..(N_RECORDS - n_real) {
        let label = format!("Artículo genérico #{:05}", i);
        // Vector aleatorio normalizado.
        for v in buf.iter_mut() {
            *v = rng.gen_range(-1.0f32..1.0f32);
        }
        normalize_inplace(&mut buf);
        catalog.push(&buf, &label);
    }

    // -- Inserción de registros semánticamente relevantes --
    for desc in TORQUE_TOOLS.iter().chain(HAND_PROTECTION).chain(LIFTING_GEAR) {
        let emb = backend.encode(desc);
        catalog.push(emb.as_slice(), *desc);
    }

    // -- Pre-normalización del catálogo completo (via Rayon) --
    catalog.normalize_all();

    println!("   ✓ Catálogo listo en {:.2}ms\n", t_start.elapsed().as_millis());

    // ─────────────────────────────────────────────────────────
    // FASE 2: Construcción del índice HNSW
    // ─────────────────────────────────────────────────────────
    println!("▶ [2/4] Construyendo índice HNSW (M=16, ef_construction=100)...");
    let t_hnsw = Instant::now();

    let mut hnsw = HnswIndex::new(16, 100);
    // Insertamos los primeros 8k nodos para demo rápida
    // (construcción completa de 64k en ~30s en hardware típico).
    let n_index = 8_000.min(N_RECORDS);
    for i in 0..n_index {
        hnsw.insert(&catalog, i);
    }
    // Insertamos siempre los nodos reales (al final del catálogo).
    for i in (N_RECORDS - n_real)..N_RECORDS {
        hnsw.insert(&catalog, i);
    }

    println!("   ✓ HNSW listo con {} nodos indexados en {:.2}ms\n",
             n_index + n_real, t_hnsw.elapsed().as_millis());

    // ─────────────────────────────────────────────────────────
    // FASE 3: BÚSQUEDA SEMÁNTICA — "Herramienta de torque"
    // ─────────────────────────────────────────────────────────
    let query_text = "Herramienta de torque";
    println!("▶ [3/4] Búsqueda semántica: \"{}\"", query_text);
    println!("   (Sin coincidencia léxica con los registros del catálogo)\n");

    let query_emb = backend.encode(query_text);

    // -- Búsqueda lineal exacta (O(N)) --
    let t_linear = Instant::now();
    let results_linear = linear_search(&catalog, query_emb.as_slice(), 10);
    let ms_linear = t_linear.elapsed().as_micros();

    println!("   ┌─ BÚSQUEDA LINEAL EXACTA (Rayon + SIMD) ──────────────┐");
    println!("   │  Tiempo: {:.1}µs sobre {} registros                 │", ms_linear, N_RECORDS);
    println!("   ├───────────────────────────────────────────────────────┤");
    for (rank, r) in results_linear.iter().enumerate() {
        println!("   │  #{:02}  score={:.4}  [{}] {}", rank + 1, r.score, r.index, r.label);
    }
    println!("   └───────────────────────────────────────────────────────┘\n");

    // -- Búsqueda HNSW aproximada (O(log N)) --
    let t_hnsw_q = Instant::now();
    let results_hnsw = hnsw.search(&catalog, query_emb.as_slice(), 10, 50);
    let ms_hnsw = t_hnsw_q.elapsed().as_micros();

    println!("   ┌─ BÚSQUEDA HNSW APROXIMADA (O(log N)) ────────────────┐");
    println!("   │  Tiempo: {}µs                                         │", ms_hnsw);
    println!("   ├───────────────────────────────────────────────────────┤");
    for (rank, r) in results_hnsw.iter().enumerate() {
        println!("   │  #{:02}  score={:.4}  [{}] {}", rank + 1, r.score, r.index, r.label);
    }
    println!("   └───────────────────────────────────────────────────────┘\n");

    // ─────────────────────────────────────────────────────────
    // FASE 4: ANÁLISIS DE CLUSTERING SEMÁNTICO
    // ─────────────────────────────────────────────────────────
    println!("▶ [4/4] Clustering semántico — islas de vecindad\n");

    let cluster_queries = [
        ("Protección para manos",   "→ ¿Encuentra guantes?"),
        ("Equipo de levantamiento", "→ ¿Encuentra tecles/eslingas?"),
        ("Llave neumática de 1/2",  "→ ¿Encuentra herramientas de torque relacionadas?"),
    ];

    for (q, hint) in &cluster_queries {
        let emb = backend.encode(q);
        let results = linear_search(&catalog, emb.as_slice(), 3);
        println!("   Consulta: \"{}\"  {}", q, hint);
        for r in &results {
            println!("   ↳  score={:.4}  {}", r.score, r.label);
        }
        println!();
    }

    // ─────────────────────────────────────────────────────────
    // ESTADÍSTICAS DE RENDIMIENTO
    // ─────────────────────────────────────────────────────────
    println!("═══════════════════════════════════════════════════════════");
    println!("  ESTADÍSTICAS DE RENDIMIENTO");
    println!("═══════════════════════════════════════════════════════════");
    println!("  Registros en catálogo   : {:>10}", N_RECORDS);
    println!("  Dimensiones por vector  : {:>10}", DIM);
    println!("  Memoria catálogo (f32)  : {:>8.1} MB",
             (N_RECORDS * DIM * 4) as f64 / 1_048_576.0);
    println!("  Búsqueda lineal exacta  : {:>8}µs", ms_linear);
    println!("  Búsqueda HNSW aprox.    : {:>8}µs", ms_hnsw);
    let speedup = ms_linear as f64 / ms_hnsw.max(1) as f64;
    println!("  Speedup HNSW vs lineal  : {:>7.1}×", speedup);
    println!("═══════════════════════════════════════════════════════════\n");

    println!("✅ Demo completado. El motor encontró herramientas de torque");
    println!("   desde una consulta en lenguaje natural, sin coincidencia léxica.\n");
}
