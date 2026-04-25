//! Motor optimizado de similitud Jaro-Winkler para deduplicación de catálogos.
//! 
//! Diseñado para strings cortos (< 64 grafemas) utilizando bitmasking (`u64`) 
//! y `ArrayVec` para lograr seguimiento de coincidencias con Zero-Allocation.

use unicode_segmentation::UnicodeSegmentation;
use arrayvec::ArrayVec;

/// Puntuación de similitud en formato de precisión doble.
pub type SimilarityScore = f64;

/// Errores de dominio para el motor Jaro-Winkler.
#[derive(Debug, PartialEq)]
pub enum JaroWinklerError {
    InvalidScaleFactor,
    InvalidPrefixLength,
    EmptyString,
    /// El motor está optimizado para u64 bitmasks (cadenas cortas).
    InputTooLong, 
}

/// Configuración del algoritmo Jaro-Winkler.
#[derive(Debug, Clone, Copy)]
pub struct JaroWinklerMatcher {
    /// Factor de escala Winkler (típicamente 0.1).
    p: f64,
    /// Límite máximo del prefijo a evaluar (típicamente 4).
    l: usize,
}

impl JaroWinklerMatcher {
    /// Construye una nueva instancia del motor.
    /// 
    /// # Errores
    /// Retorna error si `p` no está entre 0.0 y 0.25, o si `l` es mayor a 4.
    pub fn new(p: f64, l: usize) -> Result<Self, JaroWinklerError> {
        if !(0.0..=0.25).contains(&p) {
            return Err(JaroWinklerError::InvalidScaleFactor);
        }
        if l > 4 {
            return Err(JaroWinklerError::InvalidPrefixLength);
        }
        Ok(Self { p, l })
    }

    /// Normaliza un SKU eliminando guiones y espacios en blanco.
    /// Útil para la etapa de pre-procesamiento en SKU Linkage.
    pub fn normalize_sku(sku: &str) -> String {
        sku.replace(|c: char| c == '-' || c.is_whitespace(), "")
    }

    /// Calcula la similitud Jaro-Winkler entre dos cadenas.
    pub fn similarity(&self, s1: &str, s2: &str) -> Result<SimilarityScore, JaroWinklerError> {
        // 1. Descomposición de Graphemes (Unicode-aware) con ArrayVec para zero-allocation
        let mut seq1 = ArrayVec::<&str, 64>::new();
        let mut seq2 = ArrayVec::<&str, 64>::new();
        
        for g in s1.graphemes(true) {
            if seq1.try_push(g).is_err() {
                return Err(JaroWinklerError::InputTooLong);
            }
        }
        for g in s2.graphemes(true) {
            if seq2.try_push(g).is_err() {
                return Err(JaroWinklerError::InputTooLong);
            }
        }

        let len1 = seq1.len();
        let len2 = seq2.len();

        // 2. Manejo de casos triviales y límites del hardware (Bitmask u64)
        if len1 == 0 && len2 == 0 { return Ok(1.0); }
        if len1 == 0 || len2 == 0 { return Err(JaroWinklerError::EmptyString); }
        // Nota: El límite de 64 ya está garantizado por ArrayVec arriba

        // 3. Early Exit Matemático
        let max_len = len1.max(len2);
        let window = (max_len / 2).saturating_sub(1);
        let len_diff = len1.abs_diff(len2);

        // Si la diferencia de longitud es más del doble de la ventana, es matemáticamente
        // imposible alcanzar un score significativo. Cortocircuitamos a 0.0.
        if window > 0 && len_diff >= window * 2 {
            return Ok(0.0);
        }

        // 4. Zero-Allocation Match Tracking usando Bitmasks (u64)
        let mut match_mask1 = 0u64;
        let mut match_mask2 = 0u64;
        let mut matches = 0usize;

        // Doble barrido rápido
        for i in 0..len1 {
            let start = i.saturating_sub(window);
            let end = (i + window + 1).min(len2);

            for j in start..end {
                // Si el bit j no está seteado y los grafemas coinciden
                if (match_mask2 & (1 << j)) == 0 && seq1[i] == seq2[j] {
                    match_mask1 |= 1 << i;
                    match_mask2 |= 1 << j;
                    matches += 1;
                    break;
                }
            }
        }

        if matches == 0 { return Ok(0.0); }

        // 5. Cálculo exacto de Transposiciones usando los Bitmasks
        let mut transpositions = 0;
        let mut k = 0;

        for i in 0..len1 {
            if (match_mask1 & (1 << i)) != 0 {
                // Avanzar k hasta el próximo match en s2
                while k < len2 && (match_mask2 & (1 << k)) == 0 {
                    k += 1;
                }
                // Si los caracteres matcheados difieren, es una transposición
                if seq1[i] != seq2[k] {
                    transpositions += 1;
                }
                k += 1;
            }
        }
        
        let transpositions = transpositions / 2;

        // 6. Cálculo Jaro Base (precisión f64)
        let m_f = matches as f64;
        let jaro = (
            m_f / (len1 as f64) + 
            m_f / (len2 as f64) + 
            (m_f - transpositions as f64) / m_f
        ) / 3.0;

        // 7. Modificador de Prefijo Winkler
        let mut prefix = 0;
        let limit = self.l.min(len1).min(len2);
        
        for i in 0..limit {
            if seq1[i] == seq2[i] {
                prefix += 1;
            } else {
                break;
            }
        }

        let score = jaro + (prefix as f64) * self.p * (1.0 - jaro);
        
        // Capping defensivo a 1.0
        Ok(score.min(1.0))
    }
}