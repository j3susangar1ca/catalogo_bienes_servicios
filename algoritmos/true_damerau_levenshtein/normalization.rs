use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use crate::error::Result;

/// Normaliza texto a forma NFC para consistencia en comparaciones.
///
/// # Arguments
///
/// * `input` - String a normalizar
///
/// # Returns
///
/// String normalizado en forma NFC (Canonical Composition).
///
/// # Examples
///
/// ```
/// use fuzzy_search_engine::prelude::normalize_nfc;
/// let normalized = normalize_nfc("café");
/// assert_eq!(normalized, "café");
/// ```
#[inline]
pub fn normalize_nfc(input: &str) -> String {
    input.nfc().collect()
}

/// Extrae clusters de grafemas como vector de strings para procesamiento preciso.
///
/// Evita roturas en caracteres multiocteto (emojis, acentos combinados) [[21]][[26]].
///
/// # Arguments
///
/// * `input` - String normalizado previamente
///
/// # Returns
///
/// Vector de clusters de grafemas como strings individuales.
///
/// # Examples
///
/// ```
/// use fuzzy_search_engine::prelude::to_grapheme_clusters;
/// let clusters = to_grapheme_clusters("👨‍💻");
/// assert_eq!(clusters.len(), 1); // Emoji compuesto como un solo cluster
/// ```
pub fn to_grapheme_clusters(input: &str) -> Vec<String> {
    input.graphemes(true).map(|g| g.to_string()).collect()
}

/// Preprocesa texto para búsqueda: normaliza + extrae grafemas + lowercase.
///
/// Optimizado para evitar allocations múltiples mediante reutilización de buffer.
///
/// # Arguments
///
/// * `input` - Texto crudo del catálogo
/// * `buffer` - Buffer reutilizable para clusters (optimización de memoria)
///
/// # Returns
///
/// Vector de strings preprocesados listos para cálculo de distancia.
pub fn preprocess_text<'a>(
    input: &str,
    buffer: &'a mut Vec<String>,
) -> Result<&'a [String]> {
    buffer.clear();
    let normalized = normalize_nfc(input);
    buffer.extend(
        normalized
            .to_lowercase()
            .graphemes(true)
            .map(|g| g.to_string())
    );
    Ok(buffer)
}

/// Verifica si un carácter es numérico ASCII [0-9] para ponderación especial.
#[inline]
pub fn is_numeric_ascii(c: char) -> bool {
    c.is_ascii_digit()
}

/// Verifica si un carácter es alfabético ASCII para transposiciones estándar.
#[inline]
pub fn is_alphabetic_ascii(c: char) -> bool {
    c.is_ascii_alphabetic()
}