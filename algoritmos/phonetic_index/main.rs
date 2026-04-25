use std::hint::black_box;
use std::time::Instant;

use phonetic_index::index::PhoneticIndex;
use phonetic_index::phonetic_core::{DoubleMetaphone, PhoneticEncoder};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Phonetic Index Engine — Double Metaphone + Adaptación Español  ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // ── 1. Generar catálogo de 64 000 productos ────────────
    let catalog = generate_catalog(64_000);
    println!("▸ Catálogo: {} productos generados", catalog.len());

    // ── 2. Construir índice fonético (Rayon paralelo) ──────
    let start = Instant::now();
    let index = PhoneticIndex::build(&catalog);
    let build_time = start.elapsed();
    println!(
        "▸ Índice construido en {:?} (Rayon paralelo)",
        build_time
    );
    println!("  Claves primarias únicas:   {}", index.primary_key_count());
    println!("  Claves secundarias únicas: {}", index.secondary_key_count());
    println!("  Total registros:           {}", index.total_records());

    // ── 3. Benchmark de rendimiento ────────────────────────
    let encoder = DoubleMetaphone::new();
    let iters: u64 = 100_000;

    // Warm-up
    for _ in 0..2_000 {
        black_box(encoder.encode_double("Caterpillar Industrial Premium 42"));
    }

    let start = Instant::now();
    for _ in 0..iters {
        black_box(encoder.encode_double("Caterpillar Industrial Premium 42"));
    }
    let avg_ns = start.elapsed().as_nanos() / iters as u128;
    println!(
        "\n▸ Rendimiento codificación: {} ns/ término (objetivo: <500 ns) {}",
        avg_ns,
        if avg_ns < 500 { "✓" } else { "✗" }
    );

    // ── 4. Demostración de codificaciones ──────────────────
    println!("\n── Tabla de Codificaciones ───────────────────────────────────────");
    let demo_terms = [
        "Caterpillar",
        "Katerpilar",
        "Cayerpilar",
        "Gasket",
        "Gasquet",
        "Gasquetes",
        "Válvula",
        "Balvula",
        "Valbula",
        "Llanta",
        "Yanta",
        "Schmidt",
        "Smith",
        "Smyth",
        "Hernandez",
        "Ernandez",
        "Bosch",
        "Bosh",
    ];

    println!(
        "  {:<22} {:>8} {:>8}",
        "Término", "Primario", "Secund."
    );
    println!("  {:─<22} {:─>8} {:─>8}", "", "", "");
    for term in &demo_terms {
        let (p, s) = encoder.encode_double(term);
        println!("  {:<22} {:>8} {:>8}", term, p, s);
    }

    // ── 5. Fuzzy Brand Matcher ─────────────────────────────
    println!("\n── Fuzzy Brand Matcher ───────────────────────────────────────────");
    let brand_queries = ["Caterpillar", "Cayerpilar", "Katerpilar", "Bosch", "Vosch"];
    for q in &brand_queries {
        let results = index.search(q);
        let names: Vec<&str> = results
            .iter()
            .take(5)
            .map(|&id| catalog[id as usize].1.as_str())
            .collect();
        println!(
            "  '{}' → {} coincidencias{}",
            q,
            results.len(),
            if results.len() > 0 { ":" } else { "" }
        );
        for name in &names {
            println!("    · {}", name);
        }
    }

    // ── 6. Sound-Alike Normalizer ──────────────────────────
    println!("\n── Sound-Alike Normalizer ────────────────────────────────────────");
    let groups: &[&[&str]] = &[
        &["Gasket", "Gasquet", "Gasquetes"],
        &["Válvula", "Balvula", "Valbula"],
        &["Llanta", "Yanta"],
        &["Smyth", "Smith"],
    ];
    for group in groups {
        let codes: Vec<(String, String, String)> = group
            .iter()
            .map(|t| {
                let (p, s) = encoder.encode_double(t);
                (t.to_string(), p.to_string(), s.to_string())
            })
            .collect();

        let all_same_pri = codes.iter().all(|(_, p, _)| p == &codes[0].1);
        let status = if all_same_pri { "✓ GRUPO" } else { "~ PARCIAL" };

        println!("  [{}]", status);
        for (term, pri, sec) in &codes {
            println!("    {:<16} pri={:<8} sec={}", term, pri, sec);
        }
    }

    // ── 7. Voice Search Bridge ─────────────────────────────
    println!("\n── Voice Search Bridge ───────────────────────────────────────────");
    let voice_queries = [
        "filtro de aceite",
        "rodamiento SKF",
        "bomba hidraulica",
        "valvula de escape",
    ];
    for q in &voice_queries {
        let start = Instant::now();
        let results = index.search(q);
        let elapsed = start.elapsed();
        println!("  '{}' → {} coincidencias en {:?}", q, results.len(), elapsed);
        for &id in results.iter().take(3) {
            println!("    [{}] {}", id, catalog[id as usize].1);
        }
        if results.len() > 3 {
            println!("    … y {} más", results.len() - 3);
        }
    }

    // ── 8. Fuzzy Search demo ───────────────────────────────
    println!("\n── Fuzzy Search (distancia ≤ 1) ─────────────────────────────────");
    let fuzzy_queries = ["Cayerpilar", "Katerpilar", "Gasquetes"];
    for q in &fuzzy_queries {
        let start = Instant::now();
        let results = index.fuzzy_search(q, 1);
        let elapsed = start.elapsed();
        println!("  '{}' → {} coincidencias en {:?}", q, results.len(), elapsed);
        for &(id, dist) in results.iter().take(5) {
            println!("    [dist={}] [{}] {}", dist, id, catalog[id as usize].1);
        }
    }

    // ── 9. Suite de pruebas ────────────────────────────────
    println!("\n── Suite de Pruebas ──────────────────────────────────────────────");
    run_test_suite(&encoder);

    println!("\n✓ Motor listo para producción.");
}

// ─── Generación del catálogo ─────────────────────────────────

fn generate_catalog(size: usize) -> Vec<(u32, String)> {
    let brands = [
        "Caterpillar", "Komatsu", "John Deere", "Bosch", "SKF", "Timken",
        "Parker", "Eaton", "Gates", "Dayco", "NGK", "Denso", "Delphi",
        "Valeo", "Sachs", "Brembo", "Bilstein", "Monroe", "KYB", "Gabriel",
        "Mann Filter", "Wix", "Fram", "Purolator", "KN", "Baldwin",
        "Donaldson", "Cummins", "Detroit", "Volvo", "Scania", "Mercedes",
        "Renault", "Iveco", "DAF", "MAN", "Hino", "Isuzu", "Mitsubishi",
        "Nissan",
    ];

    let categories = [
        "Filtro", "Rodamiento", "Bomba", "Valvula", "Junta", "Correa",
        "Piston", "Cilindro", "Empaque", "Reten", "Cojinete", "Engrane",
        "Sensor", "Actuador", "Termostato", "Radiador", "Alternador",
        "Motor Arranque", "Inyector", "Turbo",
    ];

    let modifiers = [
        "Industrial",
        "Heavy Duty",
        "Premium",
        "Standard",
        "OEM",
        "Universal",
        "Compacto",
        "Reforzado",
        "Alta Precision",
        "Baja Emision",
    ];

    let mut catalog: Vec<(u32, String)> = Vec::with_capacity(size + 20);
    let mut id: u32 = 0;

    'outer: for brand in &brands {
        for category in &categories {
            for modifier in &modifiers {
                for num in 1u32..=8 {
                    if catalog.len() >= size {
                        break 'outer;
                    }
                    catalog.push((
                        id,
                        format!("{} {} {} {}", brand, category, modifier, num),
                    ));
                    id += 1;
                }
            }
        }
    }

    // Entradas de prueba específicas para las demos
    let extras = [
        "Katerpilar Filtro Premium 1",
        "Cayerpilar Filtro Premium 1",
        "Gasquet Junta Standard 1",
        "Gasquetes Junta Standard 1",
        "Balvula Valvula Industrial 1",
        "Valbula Valvula Industrial 1",
        "Yanta Correa Premium 1",
        "Bosh Filtro Standard 1",
        "Vosch Filtro Standard 1",
        "Ernandez Engrane OEM 1",
        "Smyth Sensor Industrial 1",
    ];
    for name in &extras {
        catalog.push((id, name.to_string()));
        id += 1;
    }

    catalog
}

// ─── Suite de pruebas en runtime ─────────────────────────────

fn run_test_suite(encoder: &DoubleMetaphone) {
    let mut passed = 0u32;
    let mut failed = 0u32;

    macro_rules! assert_test {
        ($name:expr, $cond:expr) => {
            if $cond {
                println!("  ✓ {}", $name);
                passed += 1;
            } else {
                println!("  ✗ {}", $name);
                failed += 1;
            }
        };
    }

    let enc = |s: &str| encoder.encode_double(s);

    let (p, _) = enc("Smyth");
    let (q, _) = enc("Smith");
    assert_test!("Smyth == Smith (primario)", p == q);

    let (p, _) = enc("Válvula");
    let (q, _) = enc("Balvula");
    assert_test!("Válvula == Balvula (B=V)", p == q);

    let (p, _) = enc("Gasket");
    let (q, _) = enc("Gasquet");
    assert_test!("Gasket == Gasquet (silenciosa)", p == q);

    let (p, _) = enc("Llanta");
    let (q, _) = enc("Yanta");
    assert_test!("Llanta == Yanta (LL=Y)", p == q);

    let (_, sec_cat) = enc("Caterpillar");
    let (pri_kat, _) = enc("Katerpilar");
    assert_test!(
        "Caterpillar (sec) ↔ Katerpilar (pri) [C/K + LL/L]",
        sec_cat == pri_kat
    );

    let (p, _) = enc("Hernandez");
    let (q, _) = enc("Ernandez");
    assert_test!("Hernandez == Ernandez (H muda)", p == q);

    let (p, _) = enc("Bosch");
    let (q, _) = enc("Bosh");
    assert_test!("Bosch == Bosh (CH ≈ H)", p == q);

    let (p, _) = enc("");
    assert_test!("Entrada vacía → código vacío", p.is_empty());

    let (p, _) = enc("A");
    assert_test!("Vocal sola → A", p.as_str() == "A");

    println!("\n  Resultado: {} pasadas, {} fallidas", passed, failed);
}
