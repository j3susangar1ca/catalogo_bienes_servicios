#![feature(portable_simd)]
use std::simd::prelude::*;
fn main() {
    let _ = Simd::<u64, 8>::LANES;
}
