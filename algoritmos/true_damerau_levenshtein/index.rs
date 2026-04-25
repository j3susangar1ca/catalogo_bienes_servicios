//! Estructuras de indexación para espacios métricos (ej: BK-Tree).
//!
//! Implementa el árbol de Burkhard-Keller para búsquedas eficientes.

use rayon::prelude::*;
use crate::metric::DistanceMetric;
use crate::types::Similarity;
use crate::prelude::preprocess_text;

/// Nodo mutable del BK-Tree para acceso thread-safe con construcción eficiente.
///
/// Diseñado para mutación in-place durante la construcción usando `RefCell`,
/// luego convertido a estructura inmutable para lecturas paralelas sin locks.
#[derive(Debug)]
struct Node<T> {
    /// Valor almacenado (preprocesado para búsquedas)
    value: Vec<String>,
    /// Payload asociado al valor
    payload: Option<T>,
    /// Hijos indexados por distancia desde este nodo
    children: std::collections::HashMap<u32, Box<Node<T>>>,
}

impl<T> Node<T> {
    fn new(value: Vec<String>, payload: Option<T>) -> Self {
        Self {
            value,
            payload,
            children: std::collections::HashMap::new(),
        }
    }
}

/// BK-Tree (Burkhard-Keller Tree) para búsqueda difusa en espacios métricos [[13]][[19]].
///
/// Permite búsquedas con radio dinámico basado en umbral de similitud ≥ 0.85.
///
/// # Generic Parameters
///
/// * `M` - Implementación de `DistanceMetric` (ej: `DamerauLevenshtein`)
/// * `T` - Tipo de payload asociado a cada entrada del catálogo
///
/// # Thread Safety
///
/// - Lecturas paralelas: ✅ Seguras mediante inmutabilidad post-construcción
/// - Escrituras concurrentes: ❌ No soportadas (diseño write-once, read-many)
///
/// # Performance
///
/// La construcción usa mutación in-place con `Box<Node<T>>` para evitar
/// el O(N²) por structural sharing. Las lecturas son thread-safe una vez
/// construido el árbol.
///
/// # Examples
///
/// ```rust
/// use fuzzy_search_engine::index::BKTree;
/// use fuzzy_search_engine::metric::DamerauLevenshtein;
///
/// let mut tree: BKTree<DamerauLevenshtein, String> = BKTree::new();
/// tree.insert("producto_001".to_string(), "SKU-001".to_string());
/// 
/// let results = tree.search("producto_002", Similarity::from_threshold(0.85));
/// assert!(!results.is_empty());
/// ```
pub struct BKTree<M, T> 
where
    M: DistanceMetric + Default,
{
    root: Option<Box<Node<T>>>,
    metric: M,
    /// Buffer reutilizable para normalización (optimización de allocations)
    preprocess_buffer: Vec<String>,
}

impl<M, T> BKTree<M, T>
where
    M: DistanceMetric + Default,
    T: Clone + Send + Sync,
{
    /// Crea nuevo BK-Tree vacío con métrica por defecto.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: None,
            metric: M::default(),
            preprocess_buffer: Vec::with_capacity(256), // Pre-alloc para 64k registros
        }
    }

    /// Inserta un nuevo elemento en el árbol.
    ///
    /// # Arguments
    ///
    /// * `key` - Clave de búsqueda (texto del catálogo)
    /// * `payload` - Datos asociados a retornar en resultados
    ///
    /// # Complexity
    ///
    /// O(log n) promedio para distribución uniforme de distancias.
    /// Mutación in-place para evitar O(N²) por structural sharing.
    pub fn insert(&mut self, key: String, payload: T) {
        let normalized = preprocess_text(&key, &mut self.preprocess_buffer)
            .expect("Unicode preprocessing failed")
            .to_vec();
        
        let new_node = Box::new(Node::new(normalized, Some(payload)));
        
        match &mut self.root {
            None => {
                self.root = Some(new_node);
            }
            Some(root) => {
                // Inserción iterativa in-place: evita clonar todo el camino
                Self::insert_iterative(root, new_node, &self.metric);
            }
        }
    }

    /// Inserta iterativamente manteniendo mutación in-place.
    fn insert_iterative(
        root: &mut Box<Node<T>>,
        new_node: Box<Node<T>>,
        metric: &M,
    ) {
        let mut current = root.as_mut();
        let mut node_to_insert = Some(new_node);
        
        while let Some(to_insert) = node_to_insert.take() {
            let dist = metric.distance(&current.value, &to_insert.value).raw();
            
            use std::collections::hash_map::Entry;
            match current.children.entry(dist) {
                Entry::Occupied(entry) => {
                    // Continuar descendiendo en el subárbol existente
                    current = entry.into_mut().as_mut();
                    node_to_insert = Some(to_insert);
                }
                Entry::Vacant(entry) => {
                    // Insertar directamente como hijo
                    entry.insert(to_insert);
                    break;
                }
            }
        }
    }

    /// Búsqueda difusa con umbral de similitud.
    ///
    /// # Arguments
    ///
    /// * `query` - Texto de búsqueda
    /// * `min_similarity` - Umbral mínimo para inclusión en resultados (ej: 0.85)
    ///
    /// # Returns
    ///
    /// Vector de payloads que cumplen el criterio de similitud.
    ///
    /// # Performance
    ///
    /// Complejidad promedio: O(n^0.5) para distribución uniforme [[13]].
    pub fn search(&self, query: &str, min_similarity: Similarity) -> Vec<T> {
        let query_graphemes = preprocess_text(query, &mut self.preprocess_buffer.clone())
            .expect("Query preprocessing failed")
            .to_vec();
        
        let mut results = Vec::new();
        
        if let Some(root) = &self.root {
            self.search_recursive(
                root.as_ref(),
                &query_graphemes,
                min_similarity,
                &mut results,
            );
        }
        
        results
    }

    /// Búsqueda recursiva con poda por desigualdad triangular (optimización métrica).
    fn search_recursive(
        &self,
        node: &Node<T>,
        query: &[String],
        min_similarity: Similarity,
        results: &mut Vec<T>,
    ) {
        let dist = self.metric.distance(&node.value, query);
        let similarity = Similarity::from_distance(dist, node.value.len().max(query.len()));
        
        if similarity.meets_threshold(min_similarity) {
            if let Some(ref payload) = node.payload {
                results.push(payload.clone());
            }
        }
        
        // Poda basada en desigualdad triangular: 
        // Si |d(query, node) - d(node, child)| > max_allowed_distance, podar subárbol
        let max_allowed_dist = ((1.0 - min_similarity.raw()) * 
            query.len().max(node.value.len()) as f64) as u32;
        
        for (&child_dist, child) in &node.children {
            // Aplicar desigualdad triangular para pruning
            let lower_bound = if dist.raw() > child_dist {
                dist.raw() - child_dist
            } else {
                child_dist - dist.raw()
            };
            let upper_bound = dist.raw() + child_dist;
            
            if lower_bound <= max_allowed_dist && upper_bound >= dist.raw().saturating_sub(max_allowed_dist) {
                self.search_recursive(child.as_ref(), query, min_similarity, results);
            }
        }
    }

    /// Búsqueda paralela de múltiples queries usando Rayon [[32]][[38]].
    ///
    /// # Arguments
    ///
    /// * `queries` - Slice de strings a buscar
    /// * `min_similarity` - Umbral común para todas las búsquedas
    ///
    /// # Returns
    ///
    /// Vector de vectores: resultados por cada query en orden de entrada.
    ///
    /// # Thread Safety
    ///
    /// ✅ Seguro para llamadas concurrentes: árbol inmutable + Rayon para paralelismo de datos.
    pub fn search_batch<'a>(
        &'a self,
        queries: &[&'a str],
        min_similarity: Similarity,
    ) -> Vec<Vec<T>> {
        queries
            .par_iter()
            .map(|&q| self.search(q, min_similarity))
            .collect()
    }

    /// Obtiene número de elementos indexados (operación O(n), usar con precaución).
    pub fn len(&self) -> usize {
        self.root.as_ref().map_or(0, |n| Self::count_recursive(n))
    }

    fn count_recursive(node: &Node<T>) -> usize {
        1 + node.children.values().map(|c| Self::count_recursive(c)).sum::<usize>()
    }

    /// Verifica si el árbol está vacío.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }
}

impl<M, T> Default for BKTree<M, T>
where
    M: DistanceMetric + Default,
    T: Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}