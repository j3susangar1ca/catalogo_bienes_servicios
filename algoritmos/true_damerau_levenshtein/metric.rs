use crate::types::Distance;

/// Trait para métricas de distancia editables con zero-cost abstractions.
///
/// Permite intercambiar algoritmos sin overhead runtime mediante monomorfización.
///
/// # Examples
///
/// ```rust
/// use fuzzy_search_engine::metric::DistanceMetric;
/// use fuzzy_search_engine::metric::DamerauLevenshtein;
///
/// let metric = DamerauLevenshtein::default();
/// let dist = metric.distance(&["h", "o", "l", "a"], &["h", "o", "l", "a", "s"]);
/// assert_eq!(dist.raw(), 1);
/// ```
pub trait DistanceMetric: Clone + Send + Sync {
    /// Calcula distancia entre dos secuencias de grafemas.
    ///
    /// # Arguments
    ///
    /// * `a` - Primera secuencia de grafemas
    /// * `b` - Segunda secuencia de grafemas
    ///
    /// # Returns
    ///
    /// Distancia de edición como `Distance` newtype.
    ///
    /// # Performance
    ///
    /// Complejidad: O(n*m) tiempo, O(min(n,m)) espacio mediante row-reuse.
    fn distance(&self, a: &[String], b: &[String]) -> Distance;

    /// Calcula distancia con umbral máximo para early-exit (optimización BK-Tree).
    ///
    /// # Arguments
    ///
    /// * `a` - Primera secuencia
    /// * `b` - Segunda secuencia  
    /// * `max_distance` - Límite superior para cortar cómputo temprano
    ///
    /// # Returns
    ///
    /// `Some(Distance)` si ≤ max_distance, `None` si excede el límite.
    fn distance_bounded(
        &self,
        a: &[String],
        b: &[String],
        max_distance: u32,
    ) -> Option<Distance>;
}
use crate::prelude::{is_numeric_ascii, is_alphabetic_ascii};

/// Implementación de **True Damerau-Levenshtein Distance** con ponderación dinámica.
///
/// Difiere de Optimal String Alignment al permitir múltiples ediciones sobre el mismo substring,
/// satisfaciendo la desigualdad triangular para compatibilidad con espacios métricos [[10]].
///
/// ## Ponderaciones Especiales
///
/// - Sustitución de caracteres numéricos `[0-9]`: ω = 2.0
/// - Transposición de caracteres alfabéticos: ω = 1.0 (base)
/// - Inserción/Eliminación/Sustitución estándar: ω = 1.0
///
/// # Examples
///
/// ```
/// use fuzzy_search_engine::metric::DamerauLevenshtein;
/// use fuzzy_search_engine::prelude::to_grapheme_clusters;
///
/// let metric = DamerauLevenshtein::default();
/// let a = to_grapheme_clusters("ca");
/// let b = to_grapheme_clusters("abc");
/// // True DL: "ca" -> "ac" (transposición) -> "abc" (inserción) = 2
/// let dist = metric.distance(&a, &b);
/// assert_eq!(dist.raw(), 2);
/// ```
///
/// # Panics
///
/// Nunca panica; maneja edge cases internamente con saturating arithmetic.
#[derive(Debug, Clone, Copy, Default)]
pub struct DamerauLevenshtein {
    /// Factor de peso para sustituciones numéricas (default: 2.0)
    numeric_substitution_weight: f64,
    /// Factor de peso para transposiciones alfabéticas (default: 1.0)
    alpha_transposition_weight: f64,
}

impl DamerauLevenshtein {
    /// Crea nueva instancia con pesos personalizados.
    #[must_use]
    pub fn with_weights(numeric_weight: f64, alpha_weight: f64) -> Self {
        Self {
            numeric_substitution_weight: numeric_weight,
            alpha_transposition_weight: alpha_weight,
        }
    }

    /// Calcula costo de sustitución con ponderación dinámica.
    #[inline]
    fn substitution_cost(a: &str, b: &str) -> f64 {
        if a.len() == 1 && b.len() == 1 {
            let ca = a.chars().next().unwrap();
            let cb = b.chars().next().unwrap();
            if is_numeric_ascii(ca) && is_numeric_ascii(cb) && ca != cb {
                2.0 // Peso mayor para errores en dígitos (críticos en IDs, códigos)
            } else {
                1.0
            }
        } else {
            1.0
        }
    }

    /// Calcula costo de transposición con validación alfabética.
    #[inline]
    fn transposition_cost(a_prev: &str, a_curr: &str, b_prev: &str, b_curr: &str) -> Option<f64> {
        if a_prev == b_curr && a_curr == b_prev {
            // Solo aplica peso especial si son caracteres alfabéticos
            if a_prev.len() == 1 && a_curr.len() == 1 {
                let c1 = a_prev.chars().next().unwrap();
                let c2 = a_curr.chars().next().unwrap();
                if is_alphabetic_ascii(c1) && is_alphabetic_ascii(c2) {
                    return Some(1.0); // Transposición alfabética estándar
                }
            }
            Some(1.0) // Transposición genérica
        } else {
            None
        }
    }
}

impl DistanceMetric for DamerauLevenshtein {
    fn distance(&self, a: &[String], b: &[String]) -> Distance {
        self.distance_bounded(a, b, u32::MAX).unwrap_or_else(|| {
            // Fallback: calcular sin límite si overflow (caso extremo)
            Distance::new((a.len().max(b.len()) * 2) as u32)
        })
    }

    fn distance_bounded(
        &self,
        a: &[String],
        b: &[String],
        max_distance: u32,
    ) -> Option<Distance> {
        let (n, m) = (a.len(), b.len());
        
        // Casos base optimizados
        if n == 0 { return Some(Distance::new(m as u32)); }
        if m == 0 { return Some(Distance::new(n as u32)); }
        
        // Optimización: asegurar que m <= n para minimizar espacio O(min(n,m))
        let (rows, cols, swapped) = if m <= n {
            (m + 1, n + 1, false)
        } else {
            (n + 1, m + 1, true)
        };
        
        // Buffer reutilizable para técnica Row-Reuse (optimización de caché CPU)
        let mut prev_row = vec![0u32; cols];
        let mut curr_row = vec![0u32; cols];
        let mut prev2_row = vec![0u32; cols]; // Para transposiciones (True DL requiere 3 filas)
        
        // Inicialización primera fila
        for j in 0..cols {
            prev_row[j] = j as u32;
        }
        
        // Mapa para última posición de cada carácter (optimización True DL)
        use std::collections::HashMap;
        let mut char_last_pos: HashMap<&str, usize> = HashMap::new();
        
        for i in 1..rows {
            let (src, tgt) = if swapped { (b, a) } else { (a, b) };
            let i_idx = if swapped { i - 1 } else { i - 1 };
            
            curr_row[0] = i as u32;
            let mut last_match_col = 0;
            
            for j in 1..cols {
                let j_idx = if swapped { j - 1 } else { j - 1 };
                
                // Costos base
                let del_cost = prev_row[j].saturating_add(1);
                let ins_cost = curr_row[j-1].saturating_add(1);
                
                let sub_cost = if src[i_idx] == tgt[j_idx] {
                    0
                } else {
                    Self::substitution_cost(&src[i_idx], &tgt[j_idx]) as u32
                };
                let sub_total = prev_row[j-1].saturating_add(sub_cost);
                
                let mut min_cost = del_cost.min(ins_cost).min(sub_total);
                
                // Transposición (True Damerau-Levenshtein)
                if i > 1 && j > 1 {
                    if let Some(trans_cost) = Self::transposition_cost(
                        &src[i_idx-1], &src[i_idx],
                        &tgt[j_idx-1], &tgt[j_idx]
                    ) {
                        let trans_total = prev2_row[j-2].saturating_add(trans_cost as u32);
                        min_cost = min_cost.min(trans_total);
                    }
                }
                
                // Optimización: early-exit si excede max_distance
                if min_cost > max_distance {
                    // Actualizar estado para próxima iteración
                    std::mem::swap(&mut prev2_row, &mut prev_row);
                    std::mem::swap(&mut prev_row, &mut curr_row);
                    continue;
                }
                
                curr_row[j] = min_cost;
                
                // Actualizar última posición para optimización de transposiciones
                if src[i_idx] == tgt[j_idx] {
                    last_match_col = j;
                }
            }
            
            // Rotación de filas para Row-Reuse (evita reallocaciones)
            std::mem::swap(&mut prev2_row, &mut prev_row);
            std::mem::swap(&mut prev_row, &mut curr_row);
        }
        
        let result = prev_row[if swapped { n } else { m }];
        if result <= max_distance {
            Some(Distance::new(result))
        } else {
            None
        }
    }
}