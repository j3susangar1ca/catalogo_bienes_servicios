//! Benchmarks con Criterion.rs para medir throughput y Cycles Per Byte (CPB).

use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use hamming_hpc_engine::{simd, BitMap, CatalogIndex, IdentityCode};

fn bench_popcount_xor(c: &mut Criterion) {
    let mut group = c.benchmark_group("popcount_xor_throughput");

    for size in [16, 64, 256, 1024].iter() {
        let a = vec![0xAAAAAAAAAAAAAAAAu64; *size];
        let b = vec![0x5555555555555555u64; *size];
        let bytes = (*size * 8) as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(BenchmarkId::from_parameter(bytes), &bytes, |bencher, _| {
            bencher.iter(|| simd::popcount_xor(black_box(&a), black_box(&b)).unwrap())
        });
    }
    group.finish();
}

fn bench_hamming_u8(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_u8_throughput");

    for size in [16, 32, 64, 128].iter() {
        let a = vec![0xAAu8; *size];
        let b = vec![0x55u8; *size];
        let bytes = *size as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(BenchmarkId::from_parameter(bytes), &bytes, |bencher, _| {
            bencher.iter(|| simd::hamming_distance_u8(black_box(&a), black_box(&b)).unwrap())
        });
    }
    group.finish();
}

fn bench_catalog_scan_64k(c: &mut Criterion) {
    const N: usize = 64_000;
    const ATTR_U64S: usize = 64; // 512 bytes por producto

    let mut index = CatalogIndex::with_capacity(N);
    for i in 0..N {
        let sku = IdentityCode(format!("SKU-{:08X}", i));
        let attrs = BitMap::from_u64_slice(&[i as u64; ATTR_U64S]);
        index.insert(sku, attrs);
    }
    let target = BitMap::from_u64_slice(&[0xDEADBEEF_C0FFEEEEu64; ATTR_U64S]);

    let mut group = c.benchmark_group("catalog_scan");
    group.throughput(Throughput::Elements(N as u64));
    group.sample_size(50);
    group.bench_function("64k_records_512B_attr", |b| {
        b.iter(|| index.find_by_attribute_distance(black_box(&target), black_box(100)))
    });
    group.finish();
}

criterion_group!(benches, bench_popcount_xor, bench_hamming_u8, bench_catalog_scan_64k);
criterion_main!(benches);
