// src/prelude/mod.rs
//! Re-exportes principales del motor de búsqueda difusa.

use crate::normalization;

pub use normalization::{
    normalize_nfc,
    to_grapheme_clusters,
    preprocess_text,
    is_numeric_ascii,
    is_alphabetic_ascii,
};