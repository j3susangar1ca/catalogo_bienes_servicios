use sorensen_dice_engine::index::CatalogIndex;

fn main() {
    let mut catalog = CatalogIndex::new(64_000);
    catalog.insert(1, "Bomba centrífuga de agua");
    catalog.insert(2, "Bomba centifuga");
    catalog.insert(3, "Válvula de alivio de presión de bronce");
    catalog.insert(4, "Motor eléctrico trifásico");
    catalog.insert(5, "Tubo de PVC 2 pulgadas");

    let query = "Válvula";
    let threshold = 0.1;
    println!("Consulta: '{}'", query);
    let results = catalog.search(query, threshold);
    for (id, text, score) in results {
        println!("ID: {}, Score: {:.4} -> {}", id, score, text);
    }
}
