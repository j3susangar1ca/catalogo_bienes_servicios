fn main() {
    cxx_build::bridge("src/lib.rs")
        .compile("rust_engine_bridge");

    println!("cargo:rerun-if-changed=src/lib.rs");
}
