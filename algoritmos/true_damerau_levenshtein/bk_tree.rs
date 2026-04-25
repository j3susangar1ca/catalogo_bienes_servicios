use std::sync::Arc;
use rayon::prelude::*;
use crate::metric::DistanceMetric;
use crate::types::{Distance, Similarity};
use crate::error::{FuzzySearchError, Result};
use crate::prelude::{preprocess_text, to_grapheme_clusters};

/// Nodo inmutable del BK-Tree para acceso thread-safe con `Arc`.
///
/// Diseñado para inmutabilidad post-construcción, permitiendo lecturas paralelas sin locks.
#[derive(Debug, Clone)]
struct Node<T> {
    /// Valor almacenado (preprocesado para búsquedas)
    value: Vec<String>,
    /// Payload asociado al valor
    payload: Option<T>,
    /// Hijos indexados por distancia desde este nodo
    children: std::collections::HashMap<u32, Arc<Node<T>>>,
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
/// - Lecturas paralelas: ✅ Seguras mediante `Arc<Node>` e inmutabilidad
/// - Escrituras concurrentes: ❌ No soportadas (diseño write-once, read-many)
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
    root: Option<Arc<Node<T>>>,
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
    pub fn insert(&mut self, key: String, payload: T) {
        let normalized = preprocess_text(&key, &mut self.preprocess_buffer)
            .expect("Unicode preprocessing failed")
            .to_vec();
        
        let new_node = Arc::new(Node::new(normalized, Some(payload)));
        
        match &self.root {
            None => {
                self.root = Some(new_node);
            }
            Some(root) => {
                // Inserción recursiva inmutable: crear nuevo camino sin mutar nodos existentes
                let new_root = Self::insert_recursive(
                    Arc::clone(root),
                    new_node,
                    &self.metric,
                );
                self.root = Some(new_root);
            }
        }
    }

    /// Inserta recursivamente manteniendo inmutabilidad mediante structural sharing.
    fn insert_recursive(
        current: Arc<Node<T>>,
        new_node: Arc<Node<T>>,
        metric: &M,
    ) -> Arc<Node<T>> {
        let dist = metric.distance(&current.value, &new_node.value).raw();
        
        // Clonar nodo actual para modificación inmutable
        let mut new_children = current.children.clone();
        
        match new_children.get(&dist) {
            Some(child) => {
                // Recursar en subárbol existente
                let updated_child = Self::insert_recursive(
                    Arc::clone(child),
                    new_node,
                    metric,
                );
                new_children.insert(dist, updated_child);
            }
            None => {
                // Nuevo hijo en esta distancia
                new_children.insert(dist, new_node);
            }
        }
        
        Arc::new(Node {
            value: current.value.clone(),
            payload: current.payload.clone(),
            children: new_children,
        })
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
                root,
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
        node: &Arc<Node<T>>,
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
                self.search_recursive(child, query, min_similarity, results);
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
        Self::count_recursive(&self.root)
    }

    fn count_recursive(node: &Option<Arc<Node<T>>>) -> usize {
        match node {
            None => 0,
            Some(n) => {
                1 + n.children.values().map(|c| Self::count_recursive(&Some(Arc::clone(c)))).sum::<usize>()
            }
        }
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
{
    fn default() -> Self {
        Self::new()
    }
}