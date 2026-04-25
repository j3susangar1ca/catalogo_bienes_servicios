//! Normalización de cadenas previa a la codificación fonética.
//!
//! - Minusculización
//! - Manejo de Ñ → NY  (gráfico y forma descompuesta n + tilde combinante)
//! - Desacento de vocales (á → a, é → e, …)
//! - Eliminación de todo carácter no alfabético ASCII
//! - Consciencia de grapheme clusters vía `unicode-segmentation`

use unicode_segmentation::UnicodeSegmentation;

/// Normaliza una cadena de entrada a ASCII puro en minúsculas,
/// lista para el encoder fonético.
///
/// Reglas principales:
/// - `ñ` / `Ñ` → `"ny"`
/// - Vocales acentuadas → vocal base
/// - `ç` → `"c"`, `ß` → `"ss"`
/// - Todo carácter no alfanumérico ASCII se descarta
pub fn normalize(input: &str) -> String {
    let mut result = String::with_capacity(input.len());

    for grapheme in input.graphemes(true) {
        // Lowercase de cada carácter del grapheme cluster
        let lowered: String = grapheme.chars().flat_map(|c| c.to_lowercase()).collect();

        // Detectar ñ en cualquier representación Unicode
        if lowered.contains('ñ')
            || (lowered.starts_with('n') && lowered.chars().any(|c| c == '\u{0303}'))
        {
            result.push_str("ny");
            continue;
        }

        for ch in lowered.chars() {
            match ch {
                'a'..='z' | '0'..='9' => result.push(ch),
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' | 'æ' => result.push('a'),
                'é' | 'è' | 'ê' | 'ë' => result.push('e'),
                'í' | 'ì' | 'î' | 'ï' => result.push('i'),
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'ø' => result.push('o'),
                'ú' | 'ù' | 'û' | 'ü' => result.push('u'),
                'ç' => result.push('c'),
                'ß' => {
                    result.push('s');
                    result.push('s');
                }
                _ => {} // descarta puntuación, marcas combinantes, etc. (dígitos ahora permitidos)
            }
        }
    }

    result
}

/// Versión en mayúsculas ASCII — la que consume el encoder fonético.
pub fn normalize_upper(input: &str) -> String {
    normalize(input).to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_accents() {
        assert_eq!(normalize("Válvula"), "valvula");
    }

    #[test]
    fn ene_to_ny() {
        assert_eq!(normalize("España"), "espanya");
    }

    #[test]
    fn ene_decomposed() {
        // n + combining tilde  (U+006E U+0303)
        assert_eq!(normalize("Espan\u{0303}a"), "espanya");
        // Nota: \u{0311} es inverted breve, no tilde. Probemos con tilde real:
        assert_eq!(normalize("n\u{0303}o"), "nyo");
    }

    #[test]
    fn preserve_digits() {
        assert_eq!(normalize("TORNILLO-M10X25"), "tornillom10x25");
    }

    #[test]
    fn non_alphanumeric_removed() {
        assert_eq!(normalize("SKF-6205/2RS!"), "skf62052rs");
    }

    #[test]
    fn german_eszett() {
        assert_eq!(normalize("Straße"), "strasse");
    }
}
