//! Tests de integración para el motor Hamming HPC.
//! 
//! Incluye validación de traits, detección de errores de longitud,
//! y el benchmark de integración crítico: 64,000 comparaciones < 1 ms.

use rand::Rng;
use hamming_hpc_engine::{
    BitMap, IdentityCode, CatalogIndex, HammingTarget, Result, AlignedU64x4
};

// =============================================================================
// Tests de Trait y Correctitud Funcional
// =============================================================================

#[test]
fn test_hamming_trait_u8_single_bit() -> Result<()> {
    let a = b"hello world";
    let b = b"hello worle"; // difiere en 1 byte
    let dist = a.hamming_distance(b)?;
    assert_eq!(dist, 1);
    Ok(())
}

#[test]
fn test_hamming_trait_u64_bit_pattern() -> Result<()> {
    let a = [0b1111_0000u64, 0b0000_1111u64];
    let b = [0b1110_0001u64, 0b0001_1110u64];
    // Diferencias: bit0, bit7, bit8, bit15 = 4 bits
    let dist = a.hamming_distance(&b)?;
    assert_eq!(dist, 4);
    Ok(())
}

#[test]
fn test_hamming_trait_string_ascii() -> Result<()> {
    let a = String::from("SKU-1234");
    let b = String::from("SKU-1244"); // difiere en 1 char
    let dist = a.hamming_distance(&b)?;
    assert_eq!(dist, 1);
    Ok(())
}

#[test]
fn test_incompatible_length_error() {
    let a = [1u64, 2u64];
    let b = [1u64, 2u64, 3u64];
    let result = a.hamming_distance(&b);
    assert!(result.is_err());
    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(msg.contains("IncompatibleLength"));
    }
}

#[test]
fn test_bitmap_full_popcount() -> Result<()> {
    let ones = BitMap::from_u64_slice(&[0xFFFF_FFFF_FFFF_FFFFu64; 16]);
    let zeros = BitMap::from_u64_slice(&[0x0000_0000_0000_0000u64; 16]);
    let dist = ones.hamming_distance(&zeros)?;
    assert_eq!(dist, 16 * 64); // 1024 bits distintos
    Ok(())
}

#[test]
fn test_alignment_32_bytes() {
    let block = AlignedU64x4([1, 2, 3, 4]);
    let ptr = &block as *const AlignedU64x4 as usize;
    assert_eq!(ptr % 32, 0, "AlignedU64x4 debe estar alineado a 32 bytes");
}

// =============================================================================
// Tests de Aplicación de Catálogo
// =============================================================================

#[test]
fn test_catalog_sku_typo_detection() {
    let mut index = CatalogIndex::with_capacity(100);
    index.insert(IdentityCode(String::from("ABC-1234")), BitMap::zeros(8));
    index.insert(IdentityCode(String::from("ABC-1235")), BitMap::zeros(8));
    index.insert(IdentityCode(String::from("XYZ-9999")), BitMap::zeros(8));

    // "ABC-1234" vs "ABC-1235" tienen distancia Hamming = 1
    let typos = index.find_sku_typos(&IdentityCode(String::from("ABC-1234")));
    assert_eq!(typos.len(), 1);
    assert_eq!(typos[0].sku.0, "ABC-1235");
}

#[test]
fn test_catalog_attribute_filtering() -> Result<()> {
    let mut index = CatalogIndex::with_capacity(10);
    let base = [0u64; 8];
    let mut close = [0u64; 8];
    close[0] = 0b0000_0001; // 1 bit de diferencia

    index.insert(IdentityCode(String::from("P1")), BitMap::from_u64_slice(&base));
    index.insert(IdentityCode(String::from("P2")), BitMap::from_u64_slice(&close));
    index.insert(IdentityCode(String::from("P3")), BitMap::from_u64_slice(&[0xFFu64; 8]));

    let target = BitMap::from_u64_slice(&base);
    let matches = index.find_by_attribute_distance(&target, 2);

    assert_eq!(matches.len(), 2); // P1 (dist 0) y P2 (dist 1)
    Ok(())
}

// =============================================================================
// Test Crítico de Rendimiento: 64,000 comparaciones < 1 ms
// =============================================================================

#[test]
fn test_64k_comparisons_under_1ms() {
    const N: usize = 64_000;
    const ATTR_U64S: usize = 64; // 512 bytes por registro

    let mut index = CatalogIndex::with_capacity(N);
    let mut rng = rand::thread_rng();

    // Poblar catálogo
    for i in 0..N {
        let sku = IdentityCode(format!("SKU-{:08X}", i));
        let data: Vec<u64> = (0..ATTR_U64S).map(|_| rand::Rng::gen(&mut rng)).collect();
        index.insert(sku, BitMap::from_u64_slice(&data));
    }

    let target_data: Vec<u64> = (0..ATTR_U64S).map(|_| rand::Rng::gen(&mut rng)).collect();
    let target = BitMap::from_u64_slice(&target_data);

    let start = std::time::Instant::now();
    let results = index.find_by_attribute_distance(&target, 200);
    let elapsed = start.elapsed();

    println!(
        "[INTEGRATION] Procesadas {} comparaciones en {:?} ({} matches encontrados)",
        N, elapsed, results.len()
    );

    // Assert estricto del requisito de rendimiento.
    assert!(
        elapsed.as_micros() < 1000,
        "RENDIMIENTO CRÍTICO FALLIDO: 64k comparaciones tomaron {:?}, límite < 1 ms",
        elapsed
    );
}
