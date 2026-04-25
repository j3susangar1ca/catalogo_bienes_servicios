// src/prelude/mod.rs
pub mod normalization;

pub use normalization::{
    normalize_nfc,
    to_grapheme_clusters,
    preprocess_text,
    is_numeric_ascii,
    is_alphabetic_ascii,
};