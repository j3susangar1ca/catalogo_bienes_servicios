#![feature(portable_simd)]
use std::simd::u64x4;

fn main() {
    let a = u64x4::from_array([1, 2, 3, 4]);
    let b = u64x4::from_array([4, 3, 2, 1]);
    let c = a ^ b;
    println!("{:?}", c);
}
