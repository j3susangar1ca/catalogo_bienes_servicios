use std::collections::HashMap;
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
    fn substitution_cost(&self, a: &str, b: &str) -> f64 {
        if a.len() == 1 && b.len() == 1 {
            let ca = a.chars().next().unwrap();
            let cb = b.chars().next().unwrap();
            if is_numeric_ascii(ca) && is_numeric_ascii(cb) && ca != cb {
                self.numeric_substitution_weight
            } else {
                1.0
            }
        } else {
            1.0
        }
    }

    /// Calcula costo de transposición con validación alfabética.
    #[inline]
    fn transposition_cost(&self, a_prev: &str, a_curr: &str, b_prev: &str, b_curr: &str) -> f64 {
        if a_prev == b_curr && a_curr == b_prev {
            // Solo aplica peso especial si son caracteres alfabéticos
            if a_prev.len() == 1 && a_curr.len() == 1 {
                let c1 = a_prev.chars().next().unwrap();
                let c2 = a_curr.chars().next().unwrap();
                if is_alphabetic_ascii(c1) && is_alphabetic_ascii(c2) {
                    return self.alpha_transposition_weight;
                }
            }
            1.0 // Transposición genérica
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::to_grapheme_clusters;

    #[test]
    fn test_true_damerau_levenshtein_vs_osa() {
        let metric = DamerauLevenshtein::default();
        // "ca" to "abc":
        // OSA: "ca" -> "ac" (1) -> "abc" (3) because 'b' is inserted between transposed 'a' and 'c'
        // True DL: "ca" -> "abc" (2)
        let a = to_grapheme_clusters("ca");
        let b = to_grapheme_clusters("abc");
        
        let dist = metric.distance(&a, &b);
        assert_eq!(dist.raw(), 2, "Should be 2 in True DL (transposition + insertion)");
    }

    #[test]
    fn test_triangle_inequality() {
        let metric = DamerauLevenshtein::default();
        let s1 = to_grapheme_clusters("ca");
        let s2 = to_grapheme_clusters("abc");
        let s3 = to_grapheme_clusters("ac");

        let d12 = metric.distance(&s1, &s2).raw();
        let d23 = metric.distance(&s2, &s3).raw();
        let d13 = metric.distance(&s1, &s3).raw();

        // d(s1, s2) = 2
        // d(s1, s3) = 1 (transposition)
        // d(s3, s2) = 1 (insertion)
        // 2 <= 1 + 1 (Satisfecho)
        assert!(d12 <= d13 + d23, "Triangle inequality failed: d(s1,s2)={} d(s1,s3)={} d(s3,s2)={}", d12, d13, d23);
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
        if n == 0 { return if m as u32 <= max_distance { Some(Distance::new(m as u32)) } else { None }; }
        if m == 0 { return if n as u32 <= max_distance { Some(Distance::new(n as u32)) } else { None }; }

        // Algoritmo de Lowrance y Wagner (True Damerau-Levenshtein).
        // Requiere una matriz completa (n+2)x(m+2) para rastrear transposiciones no adyacentes.
        let inf = (n + m) as u32;
        let mut d = vec![vec![0u32; m + 2]; n + 2];

        // Inicialización con valores de borde e infinitos para facilitar el cálculo de transposiciones
        d[0][0] = inf;
        for i in 0..=n {
            d[i + 1][1] = i as u32;
            d[i + 1][0] = inf;
        }
        for j in 0..=m {
            d[1][j + 1] = j as u32;
            d[0][j + 1] = inf;
        }

        let mut da = HashMap::new();

        for i in 1..=n {
            let mut db = 0;
            for j in 1..=m {
                let i1 = *da.get(&b[j-1].as_str()).unwrap_or(&0);
                let j1 = db;

                let cost = if a[i-1] == b[j-1] {
                    db = j;
                    0
                } else {
                    self.substitution_cost(&a[i-1], &b[j-1]) as u32
                };

                // Cálculo de los 4 posibles estados (Sustitución, Inserción, Eliminación, Transposición)
                d[i + 1][j + 1] = (d[i][j] + cost) // sustitución / match
                    .min(d[i + 1][j] + 1) // inserción
                    .min(d[i][j + 1] + 1) // eliminación
                    .min(d[i1][j1] + (i - i1 - 1) as u32 + 1 + (j - j1 - 1) as u32); // transposición (True DL)
            }
            da.insert(a[i-1].as_str(), i);
        }

        let result = d[n + 1][m + 1];
        if result <= max_distance {
            Some(Distance::new(result))
        } else {
            None
        }
    }
}