//! # Fuzzy Search Engine
//!
//! Motor de búsqueda difusa optimizado para baja latencia usando:
//! - **True Damerau-Levenshtein Distance** (métrica métrica válida) [[10]]
//! - **BK-Tree indexing** para búsqueda eficiente en espacios métricos [[13]][[19]]
//! - **Zero-cost abstractions** mediante traits genéricos
//!
//! ## Fórmula Matemática
//!
//! La distancia de Damerau-Levenshtein verdadera se define como:
//!
//! ```math
//! D(a,b) = \min \begin{cases}
//!   D(a_{1..n-1}, b) + \omega_{del} \\
//!   D(a, b_{1..m-1}) + \omega_{ins} \\
//!   D(a_{1..n-1}, b_{1..m-1}) + \omega_{sub} \cdot \mathbb{I}_{a_n \neq b_m} \\
//!   D(a_{1..n-2}, b_{1..m-2}) + \omega_{trans} \cdot \mathbb{I}_{a_{n-1}=b_m \land a_n=b_{m-1}}
//! \end{cases}
//! ```
//!
//! Donde $\omega_{sub} = 2.0$ para caracteres numéricos `[0-9]`, y $\omega_{trans} = 1.0$ para alfabéticos.
//!
//! ## Ejemplo de Uso
//!
//! ```rust
//! use fuzzy_search_engine::prelude::*;
//! use fuzzy_search_engine::index::BKTree;
//!
//! let mut tree = BKTree::new();
//! tree.insert("producto_123".to_string());
//! tree.insert("producto_124".to_string());
//!
//! let results = tree.search("producto_125", Similarity::from_threshold(0.85));
//! assert!(!results.is_empty());
//! ```
//!
//! # Panics
//!
//! - `panic!` si se intenta construir un `Distance` negativo
//! - `panic!` si el umbral de similitud está fuera de `[0.0, 1.0]`

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(feature = "bench", feature(test))]

pub mod normalization;
pub mod prelude;
pub mod metric;
pub mod index;
pub mod types;
pub mod error;

pub use error::FuzzySearchError;
pub use types::{Distance, Similarity};