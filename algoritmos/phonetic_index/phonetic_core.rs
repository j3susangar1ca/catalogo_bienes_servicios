//! Codificador fonético Double Metaphone con adaptaciones para español.
//!
//! ## Adaptaciones regionales
//!
//! | Fonema          | Tratamiento                                    |
//! |-----------------|------------------------------------------------|
//! | **LL**          | Código primario `Y`, secundario `L`            |
//! | **B / V**       | Ambos → `P` (fonema bilabial idéntico)         |
//! | **H** (muda)    | Silenciosa salvo en `CH` (manejado por `C`)    |
//! | **Ñ**           | Normalizado a `NY` por el módulo `normalizer`  |
//! | **Z**           | → `S` (seseo latinoamericano)                  |
//! | **G + E/I/Y**   | → `J`/`K` (gutural, coherente con jota española)|
//!
//! ## Rendimiento
//! - Zero-allocation: `ArrayString<8>` en pila, sin heap.
//! - Complejidad: O(n) donde n = longitud de la cadena normalizada.

use arrayvec::ArrayString;

use crate::normalizer;

// ─── Tipos públicos ──────────────────────────────────────────

/// Capacidad máxima del código fonético (caracteres).
pub const MAX_CODE_LEN: usize = 8;

/// Código fonético compacto, almacenado en pila.
pub type Code = ArrayString<MAX_CODE_LEN>;

/// Trait para cualquier codificador fonético.
pub trait PhoneticEncoder: Send + Sync {
    /// Devuelve únicamente el código primario.
    fn encode_primary(&self, input: &str) -> Code;
    /// Devuelve el par (primario, secundario).
    fn encode_double(&self, input: &str) -> (Code, Code);
}

// ─── Double Metaphone ────────────────────────────────────────

pub struct DoubleMetaphone {
    max_len: usize,
}

impl Default for DoubleMetaphone {
    fn default() -> Self {
        Self::new()
    }
}

impl DoubleMetaphone {
    pub fn new() -> Self {
        Self { max_len: 6 }
    }

    pub fn with_max_len(max_len: usize) -> Self {
        Self {
            max_len: max_len.min(MAX_CODE_LEN),
        }
    }

    fn encode_internal(&self, input: &str) -> (Code, Code) {
        let normalized = normalizer::normalize_upper(input);
        let b = normalized.as_bytes();
        let len = b.len();

        let mut pri = Code::new();
        let mut sec = Code::new();

        if len == 0 {
            return (pri, sec);
        }

        let max = self.max_len;
        let mut pos: usize = 0;

        // ── Condiciones de inicio ──────────────────────────
        // GN, KN, PN → silenciar la primera consonante
        // WR → silenciar W
        if len >= 2 {
            if matches!(
                (b[0], b[1]),
                (b'G', b'N') | (b'K', b'N') | (b'P', b'N') | (b'W', b'R')
            ) {
                pos = 1;
            }
        }

        // X al inicio → "S", procesar desde posición 1
        if b[0] == b'X' && pos == 0 {
            push(&mut pri, &mut sec, max, 'S', 'S');
            pos = 1;
        }

        // ── Bucle principal ────────────────────────────────
        while pos < len && pri.len() < max {
            let c = b[pos];
            let n1 = at(b, pos + 1);
            let n2 = at(b, pos + 2);
            let n3 = at(b, pos + 3);
            let p1 = if pos > 0 { b[pos - 1] } else { 0 };

            match c {
                // ── Vocales ────────────────────────────────
                b'A' | b'E' | b'I' | b'O' | b'U' => {
                    if pos == 0 {
                        push(&mut pri, &mut sec, max, 'A', 'A');
                    }
                    pos += 1;
                }

                // ── Y (híbrida: consonante/vocal) ──────────
                b'Y' => {
                    if is_pure_vowel(n1) {
                        push(&mut pri, &mut sec, max, 'Y', 'Y');
                        pos += 2;
                    } else {
                        pos += 1;
                    }
                }

                // ── B → P ──────────────────────────────────
                b'B' => {
                    push(&mut pri, &mut sec, max, 'P', 'P');
                    pos += if n1 == b'B' { 2 } else { 1 };
                }

                // ── C → reglas complejas ───────────────────
                b'C' => {
                    pos = process_c(b, pos, len, p1, n1, n2, n3, &mut pri, &mut sec, max);
                }

                // ── D → T (o J en DGE/DGY) ─────────────────
                b'D' => {
                    if n1 == b'G' && is_soft_vowel(n2) {
                        push(&mut pri, &mut sec, max, 'J', 'J');
                        pos += 3;
                    } else {
                        push(&mut pri, &mut sec, max, 'T', 'T');
                        pos += if n1 == b'D' { 2 } else { 1 };
                    }
                }

                // ── F ──────────────────────────────────────
                b'F' => {
                    push(&mut pri, &mut sec, max, 'F', 'F');
                    pos += if n1 == b'F' { 2 } else { 1 };
                }

                // ── G → reglas complejas ───────────────────
                b'G' => {
                    pos = process_g(b, pos, len, p1, n1, n2, n3, &mut pri, &mut sec, max);
                }

                // ── H → muda (español) ─────────────────────
                b'H' => {
                    pos = process_h(b, pos, len, p1, n1, n2, &mut pri, &mut sec, max);
                }

                // ── J ──────────────────────────────────────
                b'J' => {
                    push(&mut pri, &mut sec, max, 'J', 'J');
                    pos += 1;
                    while pos < len && b[pos] == b'J' {
                        pos += 1;
                    }
                }

                // ── K ──────────────────────────────────────
                b'K' => {
                    push(&mut pri, &mut sec, max, 'K', 'K');
                    pos += if n1 == b'K' { 2 } else { 1 };
                }

                // ── L (LL → Y en español) ──────────────────
                b'L' => {
                    if n1 == b'L' {
                        // LL: primario Y (español), secundario L (DM estándar)
                        push(&mut pri, &mut sec, max, 'Y', 'L');
                        pos += 2;
                    } else {
                        push(&mut pri, &mut sec, max, 'L', 'L');
                        pos += 1;
                    }
                }

                // ── M ──────────────────────────────────────
                b'M' => {
                    push(&mut pri, &mut sec, max, 'M', 'M');
                    pos += 1;
                }

                // ── N ──────────────────────────────────────
                b'N' => {
                    push(&mut pri, &mut sec, max, 'N', 'N');
                    pos += 1;
                }

                // ── P (PH → F) ─────────────────────────────
                b'P' => {
                    if n1 == b'H' {
                        push(&mut pri, &mut sec, max, 'F', 'F');
                        pos += 2;
                    } else {
                        push(&mut pri, &mut sec, max, 'P', 'P');
                        pos += 1;
                    }
                }

                // ── Q → K ──────────────────────────────────
                b'Q' => {
                    push(&mut pri, &mut sec, max, 'K', 'K');
                    pos += if n1 == b'Q' { 2 } else { 1 };
                }

                // ── R ──────────────────────────────────────
                b'R' => {
                    push(&mut pri, &mut sec, max, 'R', 'R');
                    pos += 1;
                }

                // ── S → S/X ────────────────────────────────
                b'S' => {
                    pos = process_s(b, pos, len, p1, n1, n2, n3, &mut pri, &mut sec, max);
                }

                // ── T → T/0/X ──────────────────────────────
                b'T' => {
                    pos = process_t(b, pos, len, p1, n1, n2, n3, &mut pri, &mut sec, max);
                }

                // ── V → P (español: V = B) ─────────────────
                b'V' => {
                    push(&mut pri, &mut sec, max, 'P', 'P');
                    pos += if n1 == b'V' { 2 } else { 1 };
                }

                // ── W ──────────────────────────────────────
                b'W' => {
                    pos = process_w(b, pos, len, p1, n1, n2, &mut pri, &mut sec, max);
                }

                // ── X → KS ─────────────────────────────────
                b'X' => {
                    // X al inicio ya fue manejado en condiciones de inicio
                    push_str(&mut pri, &mut sec, max, "KS", "KS");
                    pos += 1;
                }

                // ── Z → S (seseo) ──────────────────────────
                b'Z' => {
                    push(&mut pri, &mut sec, max, 'S', 'S');
                    pos += 1;
                    while pos < len && b[pos] == b'Z' {
                        pos += 1;
                    }
                }

                // ── Cualquier otro → saltar ────────────────
                _ => {
                    pos += 1;
                }
            }
        }

        (pri, sec)
    }
}

impl PhoneticEncoder for DoubleMetaphone {
    #[inline]
    fn encode_primary(&self, input: &str) -> Code {
        self.encode_internal(input).0
    }

    #[inline]
    fn encode_double(&self, input: &str) -> (Code, Code) {
        self.encode_internal(input)
    }
}

// ─── Funciones de procesamiento por consonante ───────────────

/// Procesa la letra C en contexto.
fn process_c(
    b: &[u8],
    pos: usize,
    _len: usize,
    p1: u8,
    n1: u8,
    n2: u8,
    _n3: u8,
    pri: &mut Code,
    sec: &mut Code,
    max: usize,
) -> usize {
    // SC + vocal blanda (E/I/Y): la S ya fue procesada, saltar C
    if pos >= 1 && p1 == b'S' && is_soft_vowel(n1) {
        return pos + 1;
    }

    // CIA → X/X
    if n1 == b'I' && n2 == b'A' {
        push(pri, sec, max, 'X', 'X');
        return pos + 3;
    }

    // C + vocal blanda → S/X
    if is_soft_vowel(n1) {
        push(pri, sec, max, 'S', 'X');
        return pos + 2;
    }

    // CH
    if n1 == b'H' {
        if pos == 0 {
            // CH al inicio → K/X
            push(pri, sec, max, 'K', 'X');
        } else if p1 == b'S' {
            // SCH → X/K
            push(pri, sec, max, 'X', 'K');
        } else {
            push(pri, sec, max, 'K', 'K');
        }
        return pos + 2;
    }

    // CC
    if n1 == b'C' {
        if is_soft_vowel(n2) {
            // CC + E/I/Y → KS/KS
            push_str(pri, sec, max, "KS", "KS");
            return pos + 3;
        }
        push(pri, sec, max, 'K', 'K');
        return pos + 2;
    }

    // Por defecto → K
    push(pri, sec, max, 'K', 'K');
    pos + 1
}

/// Procesa la letra G en contexto.
fn process_g(
    b: &[u8],
    pos: usize,
    len: usize,
    _p1: u8,
    n1: u8,
    n2: u8,
    _n3: u8,
    pri: &mut Code,
    sec: &mut Code,
    max: usize,
) -> usize {
    // GH
    if n1 == b'H' {
        if pos + 2 < len && !is_vowel(n2) {
            // GH + consonante → mudo
            return pos + 2;
        } else if pos + 2 < len && is_vowel(n2) {
            // GH + vocal → K
            push(pri, sec, max, 'K', 'K');
            return pos + 2;
        }
        // GH al final → mudo
        return pos + 2;
    }

    // GN → G mudo, solo N
    if n1 == b'N' {
        push(pri, sec, max, 'N', 'N');
        return pos + 2;
    }

    // G + vocal blanda (E/I/Y) → J/K
    if is_soft_vowel(n1) {
        push(pri, sec, max, 'J', 'K');
        return pos + 2;
    }

    // G + vocal no blanda (A/O/U) → K
    if is_vowel(n1) {
        push(pri, sec, max, 'K', 'K');
        return pos + 2;
    }

    // Por defecto → K
    push(pri, sec, max, 'K', 'K');
    if n1 == b'G' { pos + 2 } else { pos + 1 }
}

/// Procesa la letra H — muda en español excepto en CH (ya manejado por C).
fn process_h(
    b: &[u8],
    pos: usize,
    _len: usize,
    p1: u8,
    n1: u8,
    _n2: u8,
    pri: &mut Code,
    sec: &mut Code,
    max: usize,
) -> usize {
    // H al inicio + vocal → tratar la vocal como inicial
    if pos == 0 && is_pure_vowel(n1) {
        push(pri, sec, max, 'A', 'A');
        return pos + 2; // saltar H + vocal
    }

    // H entre vocales → muda (adaptación español)
    if is_vowel(p1) && is_pure_vowel(n1) {
        return pos + 1;
    }

    // Cualquier otra H → muda
    pos + 1
}

/// Procesa la letra S.
fn process_s(
    b: &[u8],
    pos: usize,
    _len: usize,
    _p1: u8,
    n1: u8,
    n2: u8,
    _n3: u8,
    pri: &mut Code,
    sec: &mut Code,
    max: usize,
) -> usize {
    // SH → X
    if n1 == b'H' {
        push(pri, sec, max, 'X', 'X');
        return pos + 2;
    }

    // SIO / SIA → X
    if n1 == b'I' && matches!(n2, b'A' | b'O') {
        push(pri, sec, max, 'X', 'X');
        return pos + 3;
    }

    // SW al inicio → S/X
    if pos == 0 && n1 == b'W' {
        push(pri, sec, max, 'S', 'X');
        return pos + 2;
    }

    // Por defecto → S
    push(pri, sec, max, 'S', 'S');
    pos + 1
}

/// Procesa la letra T.
fn process_t(
    b: &[u8],
    pos: usize,
    _len: usize,
    _p1: u8,
    n1: u8,
    n2: u8,
    _n3: u8,
    pri: &mut Code,
    sec: &mut Code,
    max: usize,
) -> usize {
    // TH → 0 (theta) / T
    if n1 == b'H' {
        push(pri, sec, max, '0', 'T');
        return pos + 2;
    }

    // TIO / TIA → X
    if n1 == b'I' && matches!(n2, b'A' | b'O') {
        push(pri, sec, max, 'X', 'X');
        return pos + 3;
    }

    // TCH → X
    if n1 == b'C' && n2 == b'H' {
        push(pri, sec, max, 'X', 'X');
        return pos + 3;
    }

    // Por defecto → T
    push(pri, sec, max, 'T', 'T');
    if n1 == b'T' { pos + 2 } else { pos + 1 }
}

/// Procesa la letra W.
fn process_w(
    b: &[u8],
    pos: usize,
    _len: usize,
    _p1: u8,
    n1: u8,
    n2: u8,
    pri: &mut Code,
    sec: &mut Code,
    max: usize,
) -> usize {
    // WH + vocal → A
    if n1 == b'H' && is_pure_vowel(n2) {
        push(pri, sec, max, 'A', 'A');
        return pos + 3;
    }

    // WH al final o + consonante → A/F
    if n1 == b'H' {
        push(pri, sec, max, 'A', 'F');
        return pos + 2;
    }

    // W al inicio + vocal → A
    if pos == 0 && is_pure_vowel(n1) {
        push(pri, sec, max, 'A', 'A');
        return pos + 2;
    }

    // WIC / WIT → A
    if n1 == b'I' && matches!(n2, b'C' | b'T') {
        push(pri, sec, max, 'A', 'A');
        return pos + 2;
    }

    // Por defecto → F
    push(pri, sec, max, 'F', 'F');
    pos + 1
}

// ─── Helpers ─────────────────────────────────────────────────

#[inline(always)]
fn at(b: &[u8], pos: usize) -> u8 {
    *b.get(pos).unwrap_or(&0)
}

#[inline(always)]
fn is_vowel(c: u8) -> bool {
    matches!(c, b'A' | b'E' | b'I' | b'O' | b'U' | b'Y')
}

#[inline(always)]
fn is_soft_vowel(c: u8) -> bool {
    matches!(c, b'E' | b'I' | b'Y')
}

#[inline(always)]
fn is_pure_vowel(c: u8) -> bool {
    matches!(c, b'A' | b'E' | b'I' | b'O' | b'U')
}

#[inline]
fn push(pri: &mut Code, sec: &mut Code, max: usize, p: char, s: char) {
    if pri.len() < max {
        let _ = pri.try_push(p);
    }
    if sec.len() < max {
        let _ = sec.try_push(s);
    }
}

#[inline]
fn push_str(pri: &mut Code, sec: &mut Code, max: usize, p: &str, s: &str) {
    for ch in p.chars() {
        if pri.len() < max {
            let _ = pri.try_push(ch);
        }
    }
    for ch in s.chars() {
        if sec.len() < max {
            let _ = sec.try_push(ch);
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn dm() -> DoubleMetaphone {
        DoubleMetaphone::new()
    }

    fn codes(input: &str) -> (String, String) {
        let (p, s) = dm().encode_double(input);
        (p.to_string(), s.to_string())
    }

    #[test]
    fn smyth_equals_smith() {
        let (p1, _) = codes("Smyth");
        let (p2, _) = codes("Smith");
        assert_eq!(p1, p2, "Smyth y Smith deben compartir código primario");
    }

    #[test]
    fn valvula_equals_balvula() {
        let (p1, _) = codes("Válvula");
        let (p2, _) = codes("Balvula");
        assert_eq!(p1, p2, "Válvula y Balvula deben compartir código (B=V)");
    }

    #[test]
    fn gasket_equals_gasquet() {
        let (p1, _) = codes("Gasket");
        let (p2, _) = codes("Gasquet");
        assert_eq!(p1, p2, "Gasket y Gasquet deben compartir código");
    }

    #[test]
    fn llamo_equals_yamo() {
        let (p1, _) = codes("Llamo");
        let (p2, _) = codes("Yamo");
        assert_eq!(p1, p2, "LL y Y deben producir el mismo código");
    }

    #[test]
    fn caterpillar_cross_matches_katerpilar() {
        let (_, sec_cat) = codes("Caterpillar");
        let (pri_kat, _) = codes("Katerpilar");
        // La LL en Caterpillar → Y (primario) / L (secundario)
        // Katerpilar tiene una sola L → L
        // El código secundario de Caterpillar debe coincidir con el primario de Katerpilar
        assert_eq!(
            sec_cat, pri_kat,
            "Caterpillar (sec) debe coincidir con Katerpilar (pri)"
        );
    }

    #[test]
    fn silent_h_spanish() {
        let (p1, _) = codes("Hernandez");
        let (p2, _) = codes("Ernandez");
        assert_eq!(p1, p2, "H muda: Hernandez == Ernandez");
    }

    #[test]
    fn empty_input() {
        let (p, s) = dm().encode_double("");
        assert!(p.is_empty() && s.is_empty());
    }

    #[test]
    fn single_char() {
        let (p, _) = codes("A");
        assert_eq!(p.as_str(), "A");
        let (p2, _) = codes("B");
        assert_eq!(p2.as_str(), "P");
    }

    #[test]
    fn performance_under_500ns() {
        use std::hint::black_box;
        use std::time::Instant;

        let encoder = dm();
        let iterations: u64 = 50_000;

        // Warm-up
        for _ in 0..1_000 {
            black_box(encoder.encode_double("Caterpillar Industrial Premium"));
        }

        let start = Instant::now();
        for _ in 0..iterations {
            black_box(encoder.encode_double("Caterpillar Industrial Premium"));
        }
        let elapsed = start.elapsed();
        let avg_ns = elapsed.as_nanos() / iterations as u128;

        eprintln!("  → {} ns/iteración (objetivo: <500 ns)", avg_ns);
        assert!(
            avg_ns < 500,
            "Codificación demasiado lenta: {} ns",
            avg_ns
        );
    }
}
