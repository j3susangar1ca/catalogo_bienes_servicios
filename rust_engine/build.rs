use std::env;
use std::path::PathBuf;

fn main() {
    cxx_build::bridge("src/lib.rs")
        .compile("rust_engine_bridge");

    // Exportar header generado a una ruta estable para CMake
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let header_path = out_dir.join("cxxbridge").join("rust_engine").join("src").join("lib.rs.h");
    let dest_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("include").join("rust_engine.h");

    if header_path.exists() {
        std::fs::create_dir_all(dest_path.parent().unwrap()).ok();
        std::fs::copy(header_path, dest_path).ok();
    }

    println!("cargo:rerun-if-changed=src/lib.rs");
}
