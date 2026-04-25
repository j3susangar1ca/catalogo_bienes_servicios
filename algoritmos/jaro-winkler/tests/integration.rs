use jaro_winkler_engine::*;

// Instancia estándar de la industria
fn get_matcher() -> JaroWinklerMatcher {
    JaroWinklerMatcher::new(0.1, 4).unwrap()
}

    #[test]
    fn test_brand_normalizer() {
        let matcher = get_matcher();
        
        // "Adiddas", "Adas" y "Adidas" deben vincularse fuertemente
        let score_1 = matcher.similarity("Adidas", "Adiddas").unwrap();
        let score_2 = matcher.similarity("Adidas", "Adas").unwrap();
        
        assert!(score_1 > 0.95); // Extremely high confidence
        assert!(score_2 > 0.88); // High confidence
        
        // Marcas distintas deben fallar
        let score_fail = matcher.similarity("Nike", "Adidas").unwrap();
        assert_eq!(score_fail, 0.0);
    }

    #[test]
    fn test_sku_linkage() {
        let matcher = get_matcher();
        
        let sku_raw = "ABC-123-XYZ";
        let sku_messy = "ABC 123 XYZ";
        
        let norm1 = JaroWinklerMatcher::normalize_sku(sku_raw);
        let norm2 = JaroWinklerMatcher::normalize_sku(sku_messy);
        
        let score = matcher.similarity(&norm1, &norm2).unwrap();
        assert_eq!(score, 1.0); // Perfect match post-normalization
    }

    #[test]
    fn test_unicode_awareness() {
        let matcher = get_matcher();
        
        // Verifica que los caracteres con tildes y eñes se traten como 1 unidad
        let score = matcher.similarity("Niño", "Nino").unwrap();
        assert!(score > 0.80 && score < 1.0);
        
        // Café vs Cafe
        let score_cafe = matcher.similarity("Café", "Cafe").unwrap();
        assert!(score_cafe > 0.85);
    }

    #[test]
    fn test_caterpillar_vs_cat() {
        let matcher = get_matcher();
        let score = matcher.similarity("Caterpillar", "Cat").unwrap();
        // Coincidencia exacta del prefijo, pero gran castigo por longitud
    }