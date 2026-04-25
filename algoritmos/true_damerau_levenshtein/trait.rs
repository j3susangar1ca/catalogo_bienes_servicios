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