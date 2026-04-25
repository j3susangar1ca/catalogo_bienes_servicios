<div align="center">

# Catálogo de Bienes y Servicios

**Motor de Búsqueda Difusa de Alto Rendimiento — Arquitectura Multi-Algoritmo con Paralelización SIMD**

![Rust](https://img.shields.io/badge/Rust-Nightly-dea584?logo=rust&logoColor=white)
![C++20](https://img.shields.io/badge/C%2B-20-0599C?logo=cplusplus&logoColor=white)
![Qt6](https://img.shields.io/badge/Qt6-QML-41CD52?logo=qt&logoColor=white)
![License](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue)
![Algorithms](https://img.shields.io/badge/Algorithms-7-orange)
![SIMD](https://img.shields.io/badge/SIMD-AVX2%20%7C%20NEON-ff660)
![Records](https://img.shields.io/badge/Catalog-65%2C900%2B%20registros-green)

</div>

---

## Abstract

Motor de búsqueda difusa multi-algoritmo para catálogos de bienes y servicios gubernamentales (65,900+ registros). La arquitectura integra siete métricas de similitud complementarias —Hamming, Sørensen-Dice, Double Metaphone, Damerau-Levenshtein, Jaccard, Jaro-Winkler y Coseno HNSW— bajo un orquestador central Rust con despacho Rayon y bridge FFI `cxx` hacia un frontend Qt6/QML.

---

## Arquitectura del Sistema

```mermaid
graph TB
    subgraph Frontend["Capa de Presentación — Qt6/QML"]
        QML["Main.qml<br/><i>Kirigami + Acrylic Blur</i>"]
        SM["SearchModel (C++)<br/><i>QAbstractListModel</i>"]
    end

    subgraph Bridge["Capa de Orquestación — FFI Bridge"]
        SE["rust_engine<br/><i>cxx::bridge → C++</i>"]
        CSV["CSV Loader<br/><i>65,900 registros en RAM</i>"]
        RAYON["Rayon ParIterator<br/><i>Parallel Dispatch</i>"]
    end

    subgraph Algorithms["Capa Algorítmica — 7 Crates Rust"]
        H["Hamming<br/><i>AVX-512/AVX2/NEON</i>"]
        SD["Sørensen-Dice<br/><i>Bigram Shingling</i>"]
        PM["Double Metaphone<br/><i>Índice Fonético</i>"]
        DL["Damerau-Levenshtein<br/><i>BK-Tree</i>"]
        JC["Jaccard<br/><i>MinHash LSH</i>"]
        JW["Jaro-Winkler<br/><i>Bitmask Zero-Alloc</i>"]
        CS["Coseno HNSW<br/><i>f32x8 SIMD Dot</i>"]
    end

    QML -- "50ms debounce" --> SM
    SM -- "ffi::buscar()" --> SE
    SE --> CSV
    SE --> RAYON
    RAYON --> H & SD & PM & DL & JC & JW & CS
``

**Flujo de datos:** `Input → QML TextField → debounce 50ms → SearchModel::search() → ffi::buscar(query, AlgoritmoType) → Rayon par_iter → Vec<SearchResult> → QAbstractListModel → QML Delegate`

---

## Especificaciones Técnicas

### Stack de Dependencias

| Capa | Componente | Tecnología | Versión |
|---|---|
| Toolchain | Compilador Rust | `nightly` | `rust-toolchain.toml` |
| Runtime | Paralelización | Rayon | `1.10` |
| FI | Bridge Rust↔C++ | cxx | `1.0` |
| Serialización | CSV deserialization | Serde + csv | `1.3` / `1.3` |
| Frontend | UI Framework | Qt6 Quick + Kirigami | 6.x |
| Frontend | Build System | CMake | `≥ 3.25` |
| Frontend | Estándar C++ | C++20 | — |
| Frontend | Effects | Qt QuickEffects | 6.x |

### Perfil de Compilación Release

| Parámetro | Valor | Efecto |
|---|---|
| `opt-level` | `3` | Optimización máxima del compilador |
| `lto` | `fat` | Link-Time Optimization cross-crate |
| `codegen-units` | `1` | Eliminación total de código muerto |
| `panic` | `abort` | Elimina overhead de unwinding |
| C++ flags | `-march=native -O3 -pipe` | Intrínscos SIMD nativos del host |

### Algoritmos de Similitud

| Algoritmo | Crate | Métrica | Complejidad (búsqueda) | Caso de Uso |
|---|---|
| Hamming | `hamming_hpc_engine` | d_H = popcount(x⊕y) | O(N × n/8) | SKUs longitud fija, detección de typos |
| Sørensen-Dice | `sorensen_dice_engine` | 2\|A∩B\| / (\|A\|+\|B\|) | O(N × k) | Textos cortos, variaciones parciales |
| Double Metaphone | `phonetic_index` | Código fonético dual | O(1) amortizado | Búsqueda por pronunciación |
| Damerau-Levenshtein | `fuzzy_search_engine` | d_DL (con transposición) | O(N^{0.5}) promedio | OCR, errores de tipeo |
| Jaccard | `jaccard_engine` | \|A∩B\| / \|A∪B\| | O(N × k) | Bag-of-words, tokens desordenados |
| Jaro-Winkler | `jaro_winkler_engine` | d_J + lp(1−d_J) | O(N × w) | Deduplicación, SKU linkage |
| Coseno + HNSW | `semantic_engine` | a·b / (‖a‖·‖b‖) | O(ef × log N × d/8) | Búsqueda semántica con embeddings |

### Estructura del Workspace
```

```

catalogo_bienes_servicios/
├── Cargo.toml # Workspace root (8 members)
├── rust_engine/ # FFI Bridge central (cxx → C++)
│ ├── build.rs # cxx_build::bridge("src/lib.rs")
│ ├── Cargo.toml # Dependencias: 7 crates + rayon + csv
│ └── src/lib.rs # SearchMaster: CSV + despacho Rayon
├── algoritmos/
│ ├── hamming/ # SIMD popcount_xor + CatalogIndex
│ ├── sørensen_dice/ # Bigram shingling + Dice coefficient
│ ├── phonetic_index/ # Double Metaphone + Inverted Index
│ ├── true_damerau_levenshtein/ # True DL DP + BK-Tree
│ ├── jaccard/ # Tokenización + Jaccard + MinHash
│ ├── jaro-winkler/ # Bitmask match + Winkler prefix
│ └── similitud_coseno/
│ └── semantic_engine/ # 768-dim SIMD dot + HNSW ANS
├── frontend/ # Qt6/QML (CMake ≥ 3.25)
│ ├── main.cpp # Entry point + QML registration
│ ├── SearchModel.h / .cpp # QAbstractListModel ↔ Rust FFI
│ └── Main.qml # Kirigami + acrylic blur
├── catalogo.csv # Dataset: 65,900 registros
└── rust-toolchain.toml # Nightly channel

```

---

## Despliegue / Inicialización

### Prerrequisitos

| Dependencia  | Mínimo  | Verificación                          |
| ------------ | ------- | ------------------------------------- |
| Rust nightly | `1.78+` | `rustup show`                         |
| CMake        | `3.25+` | `cmake --version`                     |
| Qt6          | `6.x`   | `qmake6 --version`                    |
| KF6 Kirigami | `6.x`   | `pkg-config --modversion KF6Kirigami` |

### Compilación del Motor Rust

````bash
# Clonar el repositorio
git clone https://github.com/j3susangar1ca/catalogo_bienes_servicios.git
cd catalogo_bienes_servicios

# Compilar el workspace completo (perfil release)
cargo build --release
``

### Compilación del Frontend

```bash
cd frontend
cmake -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
````

### Ejecución

````bash
# Ejecutar el motor de búsqueda
./target/release/semantic_engine_bin

# Ejecutar el frontend Qt6/QML
./frontend/build/OmniboxLauncher
``

### Compilación con SIMD Nativo (Opcional)

```bash
# Detectar y habilitar AVX-512/AVX2/NEON del host
RUSTFLAGS="-C target-cpu=native" cargo build --release
````

---

<div align="center">

**Catálogo de Bienes y Servicios** — Motor de búsqueda difusa multi-algoritmo

</div>
```

---

**Notas sobre el contenido:**

- Todos los datos (versiones dependencias, estructura de archivos, algoritmos, configuraciones de compilación) fueron extraídos directamente del repositorio — nada fue inventado.
- Los badges usan Shields.io con logos oficiales de las tecnologías detectadas.
- El diagrama Mermaid modela las tres capas reales del sistema: Frontend (Qt6/QML) → Bridge (rust_engine + cxx) → 7 crates algorítmicos.
- La tabla de algoritmos incluye las métricas exactas implementadas, extraídas del código fuente de cada crate.
- Los comandos de compilación corresponden a los `Cargo.toml` y `CMakeLists.txt` reales del proyecto.
