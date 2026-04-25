# Catálogo de Bienes y Servicios — Motor de Búsqueda Difusa HPC

> **Documentación Técnica de Nivel Científico-Industrial**
> Arquitectura multi-algoritmo de búsqueda difusa con paralelización SIMD, índices métricos y bridge FFI Rust↔C++/Qt6.

---

## Tabla de Contenidos

1. [Visión General del Sistema](#1-visión-general-del-sistema)
2. [Arquitectura del Workspace](#2-arquitectura-del-workspace)
3. [Algoritmo 1 — Distancia de Hamming (SIMD HPC Engine)](#3-algoritmo-1--distancia-de-hamming-simd-hpc-engine)
4. [Algoritmo 2 — Coeficiente de Sørensen-Dice](#4-algoritmo-2--coeficiente-de-sørensen-dice)
5. [Algoritmo 3 — Double Metaphone (Índice Fonético)](#5-algoritmo-3--double-metaphone-índice-fonético)
6. [Algoritmo 4 — Distancia True Damerau-Levenshtein](#6-algoritmo-4--distancia-true-damerau-levenshtein)
7. [Algoritmo 5 — Coeficiente de Jaccard](#7-algoritmo-5--coeficiente-de-jaccard)
8. [Algoritmo 6 — Similitud Jaro-Winkler](#8-algoritmo-6--similitud-jaro-winkler)
9. [Algoritmo 7 — Similitud Coseno / HNSW (Semantic Engine)](#9-algoritmo-7--similitud-coseno--hnsw-semantic-engine)
10. [Motor Central — rust_engine (FFI Bridge)](#10-motor-central--rust_engine-ffi-bridge)
11. [Frontend — Qt6/QML (The Omnibox)](#11-frontend--qt6qml-the-omnibox)
12. [Análisis de Fallos, Errores y Áreas de Mejora](#12-análisis-de-fallos-errores-y-áreas-de-mejora)

---

## 1. Visión General del Sistema

El sistema implementa un motor de búsqueda difusa de alto rendimiento orientado a catálogos de bienes y servicios gubernamentales/industriales con capacidad para 64,000+ registros. La filosofía de diseño combina siete algoritmos de similitud complementarios bajo una arquitectura de microkernel paralelo:

- **Capa Algorítmica**: 7 crates Rust independientes, cada uno con su métrica de distancia/similitud especializada
- **Capa de Orquestación**: `rust_engine` como FFI bridge central que despacha consultas al algoritmo seleccionado mediante `cxx`
- **Capa de Presentación**: Qt6/QML con efecto acrylic, chips de algoritmo reactivos y debounce de 50ms

La selección de algoritmos no es arbitraria: cada uno resuelve una clase diferente de problema de similitud textual:

| Algoritmo                | Espacio Métrico | Función Objetivo                         | Caso de Uso Óptimo                            |
| ------------------------ | --------------- | ---------------------------------------- | --------------------------------------------- |
| Hamming                  | d_H ∈ ℕ         | Validación bit-a-bit                     | Detección de typos en SKUs de longitud fija   |
| Sørensen-Dice            | [0,1] ⊂ ℝ       | Superposición de n-gramas                | Textos cortos con variaciones parciales       |
| Double Metaphone         | Código fonético | Equivalencia fonémica                    | Búsqueda por pronunciación (voice-first)      |
| True Damerau-Levenshtein | d_DL ∈ ℕ        | Distancia de edición con transposiciones | OCR, errores de tipeo, variantes ortográficas |
| Jaccard                  | [0,1] ⊂ ℝ       | Intersección/Unión de tokens             | Búsqueda desordenada (bag-of-words)           |
| Jaro-Winkler             | [0,1] ⊂ ℝ       | Coincidencia con bonus de prefijo        | Deduplicación, SKU linkage                    |
| Coseno + HNSW            | [-1,1] ⊂ ℝ      | Producto punto normalizado               | Búsqueda semántica con embeddings             |

---

## 2. Arquitectura del Workspace

```
catalogo_bienes_servicios/
├── Cargo.toml                          # Workspace root (resolver = "2")
├── rust_engine/                        # FFI Bridge (cxx → C++/Qt6)
│   ├── build.rs                        # cxx_build::bridge("src/lib.rs")
│   ├── Cargo.toml                      # Depende de todos los algoritmos
│   └── src/lib.rs                      # SearchMaster: carga CSV + despacho Rayon
├── algoritmos/
│   ├── hamming/                        # simd::popcount_xor + CatalogIndex
│   ├── sørensen_dice/                  # Bigram shingling + Dice coefficient
│   ├── phonetic_index/                 # Double Metaphone + Inverted Index
│   ├── true_damerau_levenshtein/       # True DL DP + BK-Tree
│   ├── jaccard/                        # Tokenización + Jaccard exacto + MinHash
│   ├── jaro-winkler/                   # Bitmask match + Winkler prefix bonus
│   └── similitud_coseno/               # 768-dim SIMD dot + HNSW ANNS
│       └── semantic_engine/            # Sub-crate binario
└── frontend/                           # Qt6/QML (CMakeLists.txt)
    ├── main.cpp                        # Entry point + QML registration
    ├── SearchModel.h/.cpp              # QAbstractListModel ↔ Rust FFI
    └── Main.qml                        # UI Kirigami con acrylic blur
```

**Perfiles de compilación**: El workspace utiliza `opt-level = 3`, LTO fat, `codegen-units = 1` y `panic = "abort"` para el perfil release, maximizando la eliminación de código muerto y la inlinación cross-crate.

---

## 3. Algoritmo 1 — Distancia de Hamming (SIMD HPC Engine)

### 3.1 Fundamento Matemático

La distancia de Hamming entre dos cadenas de igual longitud se define como:

$$d_H(\mathbf{x}, \mathbf{y}) = \sum_{i=1}^{n} \mathbb{1}[x_i \neq y_i]$$

En su formulación sobre vectores binarios (para `BitMap`), la operación se reduce a:

$$d_H(\mathbf{x}, \mathbf{y}) = \text{popcount}(\mathbf{x} \oplus \mathbf{y})$$

donde $\oplus$ denota la operación XOR bit-a-bit y `popcount` cuenta los bits activos en el resultado.

**Propiedades formales**:

- Métrica estricta: satisface positividad ($d_H(x,y) \geq 0$), identidad de indiscernibles ($d_H(x,y) = 0 \iff x = y$), simetría ($d_H(x,y) = d_H(y,x)$), y desigualdad triangular ($d_H(x,z) \leq d_H(x,y) + d_H(y,z)$)
- Dominio: $\{0, 1, \ldots, n\}$ para vectores de $n$ bits
- Complejidad computacional: $O(n/w)$ donde $w$ es el ancho del registro SIMD (512/256/128 bits)

### 3.2 Lógica Algorítmica

La implementación sigue una estrategia de **dispatch estático en tiempo de compilación** mediante `cfg` flags, eliminando overhead de dispatch dinámico:

1. **Selection Layer**: En compilación, `cfg(target_feature)` selecciona el ancho de vector: AVX-512 (8×u64), AVX2 (4×u64), NEON (2×u64), o fallback escalar
2. **Kernel**: Itera sobre bloques de `u64`, aplicando XOR + `count_ones()` por palabra
3. **Reducción**: Suma acumulativa de los popcounts parciales

Para la variante de bytes (`hamming_distance_u8`), el proceso es análogo pero opera sobre `u8` con popcount por byte.

### 3.3 Estructura de Datos: BitMap

El `BitMap` utiliza bloques de `AlignedU64x4` (4×u64 = 32 bytes) con `#[repr(align(32))]` para garantizar que las cargas SIMD nunca crucen fronteras de línea de caché L1 (32 bytes en x86_64). La conversión a slice plano mediante `as_u64_slice()` utiliza un `unsafe` justificado: `AlignedU64x4` es `#[repr(transparent)]` sobre `[u64; 4]`, y `Vec` garantiza contigüidad.

### 3.4 Complejidad

| Operación                                  | Tiempo          | Espacio                |
| ------------------------------------------ | --------------- | ---------------------- |
| `popcount_xor`                             | O(n/8) con AVX2 | O(1)                   |
| `hamming_distance_u8`                      | O(n)            | O(1)                   |
| `find_by_attribute_distance` (N registros) | O(N × n/8)      | O(k) donde k = matches |
| `find_sku_typos` (N registros)             | O(N × len_sku)  | O(k)                   |

---

## 4. Algoritmo 2 — Coeficiente de Sørensen-Dice

### 4.1 Fundamento Matemático

El coeficiente de Sørensen-Dice mide la similitud entre dos conjuntos $A$ y $B$ mediante la fórmula:

$$DSC(A, B) = \frac{2|A \cap B|}{|A| + |B|}$$

En la implementación, los conjuntos $A$ y $B$ se construyen como **bigramas (2-gramas) de grafemas Unicode**, hasheados a `u64` con AHash:

$$A = \{h(g_i \| g_{i+1}) \mid i = 0, \ldots, |graphemes|-2\}$$

donde $h$ es la función de hash AHash y $\|$ denota concatenación. Los conjuntos resultantes se ordenan y deduplican para habilitar la intersección con marcha de dos punteros.

**Propiedades**:

- Rango: $[0, 1]$ donde 0 = disjuntos, 1 = idénticos
- No es métrica: no satisface la desigualdad triangular en general
- Sensible a la longitud: el denominador penaliza asimetría en cardinalidades

### 4.2 Lógica Algorítmica

1. **Shingler**: `generate_shingles(text)` extrae grafemas Unicode, genera ventanas deslizantes de tamaño 2, hashea cada bigrama con `AHasher`, ordena y deduplica → resultado: `Vec<u64>` ordenado
2. **Intersección**: `intersect_sorted(a, b)` implementa la técnica de marcha de dos punteros sobre los vectores ordenados en O(|A| + |B|)
3. **Scoring**: `dice_similarity(query, record, threshold)` implementa **early-exit por cota superior**: calcula el máximo DSC posible (caso donde la intersección = `min(|A|,|B|)`) y si este máximo es menor que el umbral, retorna `None` sin calcular la intersección real

### 4.3 Optimización de Cota Superior

La cota superior del DSC es:

$$DSC_{max} = \frac{2 \cdot \min(|A|, |B|)}{|A| + |B|}$$

Si $DSC_{max} < \theta$, entonces necesariamente $DSC(A,B) < \theta$, por lo que el registro se descarta sin computar la intersección. Esto reduce drásticamente el trabajo en catálogos con alta variabilidad de longitud de descripción.

### 4.4 Complejidad

| Operación                     | Tiempo                       | Espacio  |
| ----------------------------- | ---------------------------- | -------- |
| `generate_shingles`           | O(n log n) por la ordenación | O(n)     |
| `intersect_sorted`            | O(\|A\| + \|B\|)             | O(1)     |
| `dice_similarity`             | O(\|A\| + \|B\|) amortizado  | O(1)     |
| `search` (N registros, Rayon) | O(N × (k + log k))           | O(N × k) |

donde $k$ = número promedio de bigramas por registro.

---

## 5. Algoritmo 3 — Double Metaphone (Índice Fonético)

### 5.1 Fundamento Matemático

Double Metaphone es un algoritmo de codificación fonética que asigna a cada cadena un **par de códigos** $(p, s)$ donde $p$ es el código primario y $s$ el secundario, representando pronunciaciones alternativas:

$$\text{DM}: \Sigma^* \rightarrow (\Sigma_c^{\leq 8})^2$$

donde $\Sigma_c = \{A, F, J, K, L, M, N, P, R, S, T, X, 0, Y\}$ es el alfabeto de códigos.

El algoritmo opera como un **autómata finito determinista** sobre la cadena normalizada, con transiciones condicionadas por el contexto (1 carácter precedente + 3 caracteres siguientes).

**Adaptaciones para español**:

| Fonema | Regla DM estándar | Adaptación                 | Justificación fonológica                                              |
| ------ | ----------------- | -------------------------- | --------------------------------------------------------------------- |
| LL     | L                 | Primario: Y, Secundario: L | Yeísmo: /ʎ/ → /ʝ/ en la mayoría de hispanohablantes                   |
| B/V    | B → P, V → F      | Ambos → P                  | Ensordecimiento bilabial: /b/ y /β/ comparten punto de articulación   |
| H      | Consonante activa | Muda (excepto CH)          | H muda en español: /h/ = ∅ salvo en dígrafo CH                        |
| Z      | S → S             | Z → S                      | Seseo latinoamericano: /θ/ → /s/                                      |
| G+E/I  | G → J/K           | G+E/I → J/K                | Fricativización: /x/ (jota) es el alófono de G ante vocales palatales |

### 5.2 Lógica Algorítmica

1. **Normalización**: `normalizer::normalize_upper()` aplica: lowercase → mapeo de acentos → Ñ→NY → ß→SS → eliminación de no-alfabéticos
2. **Codificación**: `encode_internal()` recorre la cadena normalizada con un puntero `pos`, aplicando reglas contextuales según el carácter actual y su ventana de 4 posiciones (anterior + 3 siguientes)
3. **Almacenamiento**: Códigos en `ArrayString<8>` (pila, zero-allocation), máximo 6 caracteres por defecto
4. **Indexación**: `PhoneticIndex` construye dos hash maps invertidos: `primary_map: Code → Vec<RecordId>` y `secondary_map: Code → Vec<RecordId>`
5. **Búsqueda difusa**: `fuzzy_search()` genera variantes del código fonético a distancia de edición 1 (deletion, substitution, insertion, transposition) para encontrar coincidencias aproximadas

### 5.3 Complejidad

| Operación                             | Tiempo            | Espacio     |
| ------------------------------------- | ----------------- | ----------- |
| Codificación                          | O(n)              | O(1) (pila) |
| Construcción del índice (N registros) | O(N × L) paralelo | O(N × C)    |
| Búsqueda exacta                       | O(1) amortizado   | O(k)        |
| Búsqueda difusa (radio 1)             | O(C × 26 × L²)    | O(k)        |

donde $L$ = longitud promedio del código, $C$ = número de códigos únicos, $k$ = resultados.

---

## 6. Algoritmo 4 — Distancia True Damerau-Levenshtein

### 6.1 Fundamento Matemático

La distancia de Damerau-Levenshtein verdadera (no OSA — Optimal String Alignment) entre dos secuencias $a$ y $b$ se define recursivamente como:

$$d_{DL}(a, b) = \min \begin{cases} d_{DL}(a[1..], b) + 1 & \text{(deleción)} \\ d_{DL}(a, b[1..]) + 1 & \text{(inserción)} \\ d_{DL}(a[1..], b[1..]) + c_{sub}(a_0, b_0) & \text{(sustitución)} \\ d_{DL}(a[2..], b[2..]) + c_{trans} & \text{si } a_0 = b_1 \wedge a_1 = b_0 \text{ (transposición)} \end{cases}$$

La diferencia crítica con OSA es que True DL **permite múltiples ediciones sobre el mismo substring**, lo que satisface la desigualdad triangular:

$$d_{DL}(x, z) \leq d_{DL}(x, y) + d_{DL}(y, z)$$

Esta propiedad es **necesaria** para la correcta poda en el BK-Tree.

**Ponderaciones especiales**:

- Sustitución de dígitos numéricos: $c_{sub}(d_i, d_j) = 2.0$ si $d_i, d_j \in [0,9]$ y $d_i \neq d_j$
- Transposición alfabética: $c_{trans} = 1.0$
- Operaciones estándar: $c_{ins} = c_{del} = c_{sub} = 1.0$

La penalización de dígitos numéricos (ω=2.0) refleja el hecho de que en SKUs y códigos de producto, un error en un dígito cambia el producto completamente (e.g., "SKU-100" vs "SKU-200" son productos distintos), mientras que en texto alfabético la sustitución es menos crítica.

### 6.2 Lógica Algorítmica (Programación Dinámica)

La implementación utiliza la técnica de **row-reuse con 3 filas** para True DL:

1. **Fila actual** (`curr_row`), **fila anterior** (`prev_row`), **fila ante-anterior** (`prev2_row`) — las transposiciones requieren acceso a `dp[i-2][j-2]`
2. **Optimización de espacio**: Siempre itera con $m \leq n$ para usar $O(\min(n,m))$ espacio
3. **Early-exit**: Si el costo mínimo en una celda excede `max_distance`, se omite la actualización (optimización para BK-Tree)
4. **Mapa de posiciones**: `HashMap<&str, usize>` almacena la última posición vista de cada grafema (optimización teórica para True DL, aunque en la implementación actual se usa la verificación directa de transposición)

### 6.3 BK-Tree (Burkhard-Keller Tree)

El BK-Tree es una estructura de datos que explota la desigualdad triangular para poda en espacios métricos:

$$\text{Si } |d(q, n) - d(n, c)| > r \implies d(q, c) > r$$

donde $q$ es la query, $n$ es el nodo actual, $c$ es un hijo a distancia $d(n,c)$, y $r$ es el radio de búsqueda.

**Inserción**: Recursiva, determinística — cada elemento se inserta bajo la raíz en la posición $d_{DL}(\text{root}, \text{element})$, o recursa en el hijo existente a esa distancia.

**Búsqueda**: Recorre el árbol podando subárboles donde la desigualdad triangular garantiza que no puede haber resultados dentro del radio.

### 6.4 Complejidad

| Operación                      | Tiempo                                   | Espacio     |
| ------------------------------ | ---------------------------------------- | ----------- |
| `distance` (n×m)               | O(n×m)                                   | O(min(n,m)) |
| `distance_bounded`             | O(n×m) worst, mucho menor con early-exit | O(min(n,m)) |
| BK-Tree `insert` (N elementos) | O(N × log N × d)                         | O(N)        |
| BK-Tree `search` (radio r)     | O(N^{1-r/d\_{max}}) promedio             | O(k)        |
| `search_batch` (Q queries)     | O(Q × N^{0.5}) paralelo                  | O(Q × k)    |

---

## 7. Algoritmo 5 — Coeficiente de Jaccard

### 7.1 Fundamento Matemático

El índice de Jaccard mide la similitud entre dos conjuntos $A$ y $B$:

$$J(A, B) = \frac{|A \cap B|}{|A \cup B|} = \frac{|A \cap B|}{|A| + |B| - |A \cap B|}$$

**Convención**: $J(\emptyset, \emptyset) = 1.0$ (conjuntos vacíos son idénticos).

La **distancia de Jaccard** se define como $d_J = 1 - J(A,B)$, que sí es métrica estricta.

### 7.2 Tokenización

El pipeline de tokenización transforma texto en conjuntos ordenados de hashes:

$$\text{tokenize}(s) = \text{sort}(\{h(t) \mid t \in \text{tokens}(s) \setminus \text{stopwords}\})$$

donde:

1. Normalización NFD (descomposición canónica Unicode)
2. Segmentación por delimitadores no-alfanuméricos
3. Filtro de 35 stop-words en español
4. Hashing con AHash (semilla aleatoria por sesión)

### 7.3 MinHash (Estimación Probabilística)

MinHash genera firmas compactas para estimar Jaccard sin comparar conjuntos completos. Para $k$ funciones hash $\{h_1, \ldots, h_k\}$:

$$\text{sig}(S) = \left[\min_{s \in S} h_i(s)\right]_{i=1}^{k}$$

La estimación de Jaccard es:

$$\hat{J}(A, B) = \frac{1}{k} \sum_{i=1}^{k} \mathbb{1}[\text{sig}(A)_i = \text{sig}(B)_i]$$

**Cota de error**: Por la desigualdad de Hoeffding, el error estándar es $O(1/\sqrt{k})$. Con $k=128$, $\epsilon \approx 0.089$; con $k=256$, $\epsilon \approx 0.063$.

Las semillas se generan con un **LCG** (Linear Congruential Generator) con parámetros $a = 6364136223846793005$, $c = 1$, $m = 2^{64}$, a partir de la semilla `0xcafebabe`.

### 7.4 Complejidad

| Operación                       | Tiempo                | Espacio  |
| ------------------------------- | --------------------- | -------- |
| `tokenize_and_hash`             | O(n log n)            | O(n)     |
| `jaccard` (exacto)              | O(\|A\| + \|B\|)      | O(1)     |
| `MinHash::signature`            | O(k × n)              | O(k)     |
| `estimated_jaccard`             | O(k)                  | O(1)     |
| `Catalog::search` (N registros) | O(N × k_avg) paralelo | O(top_k) |

---

## 8. Algoritmo 6 — Similitud Jaro-Winkler

### 8.1 Fundamento Matemático

La distancia de Jaro entre dos cadenas $s_1$ y $s_2$ se define como:

$$d_{Jaro}(s_1, s_2) = \frac{1}{3} \left( \frac{m}{|s_1|} + \frac{m}{|s_2|} + \frac{m - t}{m} \right)$$

donde:

- $m$ = número de caracteres coincidentes (matching window $w = \lfloor \max(|s_1|, |s_2|) / 2 \rfloor - 1$)
- $t$ = número de transposiciones (pares de coincidencias en orden inverso, contadas como $t/2$)

La extensión Winkler aplica un bonus al prefijo común:

$$d_{JW}(s_1, s_2) = d_{Jaro} + \ell \cdot p \cdot (1 - d_{Jaro})$$

donde $\ell \leq 4$ es la longitud del prefijo común y $p \in [0, 0.25]$ es el factor de escala.

### 8.2 Lógica Algorítmica (Bitmask Zero-Allocation)

La innovación principal es el uso de **bitmasks de u64** para tracking de coincidencias, eliminando todas las asignaciones de heap:

1. **Match Phase**: Para cada posición $i$ en $s_1$, busca en la ventana $[i-w, i+w]$ de $s_2$ el primer grafema coincidente no ya emparejado. Los bits de `match_mask1` y `match_mask2` marcan posiciones emparejadas.
2. **Transposition Phase**: Recorre los bits activos de `match_mask1` en orden, avanza el puntero $k$ en `match_mask2`, y compara si los grafemas difieren (transposición).
3. **Winkler Bonus**: Cuenta prefijo común y aplica la fórmula.

**Limitación**: El bitmask de `u64` limita las cadenas a 64 grafemas. Para cadenas más largas, se retorna `InputTooLong`.

### 8.3 Early-Exit Matemático

Si la diferencia de longitudes $||s_1| - |s_2|| \geq 2w$ (donde $w$ es la ventana de matching), es matemáticamente imposible obtener $m > 0$, por lo que se cortocircuita a 0.0.

### 8.4 Complejidad

| Operación       | Tiempo         | Espacio |
| --------------- | -------------- | ------- |
| `similarity`    | O(n × w) worst | O(1)    |
| `normalize_sku` | O(n)           | O(n)    |

---

## 9. Algoritmo 7 — Similitud Coseno / HNSW (Semantic Engine)

### 9.1 Fundamento Matemático

La similitud del coseno entre dos vectores $\mathbf{a}, \mathbf{b} \in \mathbb{R}^d$ es:

$$\cos(\mathbf{a}, \mathbf{b}) = \frac{\mathbf{a} \cdot \mathbf{b}}{\|\mathbf{a}\|_2 \cdot \|\mathbf{b}\|_2} = \frac{\sum_{i=1}^{d} a_i b_i}{\sqrt{\sum_{i=1}^{d} a_i^2} \cdot \sqrt{\sum_{i=1}^{d} b_i^2}}$$

Cuando los vectores están **pre-normalizados** ($\|\mathbf{a}\|_2 = \|\mathbf{b}\|_2 = 1$), la similitud coseno se reduce al producto punto:

$$\cos(\mathbf{a}, \mathbf{b}) \equiv \mathbf{a} \cdot \mathbf{b} = \sum_{i=1}^{d} a_i b_i$$

Esta es la optimización fundamental: la normalización previa convierte O(3d) en O(d) por comparación.

### 9.2 Producto Punto SIMD

La implementación utiliza `wide::f32x8` para procesar 8 dimensiones por ciclo:

$$\text{dot\_simd}(\mathbf{a}, \mathbf{b}) = \sum_{i=0}^{d/8-1} \text{reduce}(\text{f32x8}(\mathbf{a}_{8i:8i+8}) \times \text{f32x8}(\mathbf{b}_{8i:8i+8}))$$

donde `reduce` es la suma horizontal de 8 lanes. Requiere que $d$ sea múltiplo de 8 (768 = 96 × 8 ✓).

### 9.3 HNSW (Hierarchical Navigable Small World)

HNSW es un algoritmo de búsqueda aproximada de vecinos más cercanos (ANNS) con complejidad $O(\log N)$ basado en grafos multinivel:

**Construcción**:

1. Cada nodo se asigna a una capa máxima $l_{max} = \lfloor -\ln(r) \cdot m_L \rfloor$ donde $r \sim U(0,1)$ y $m_L = 1/\ln(M)$
2. En cada capa $[0, l_{max}]$, se inserta conectando con los $M$ vecinos más cercanos encontrados por búsqueda greedy
3. Las conexiones son bidireccionales con poda si exceden $M$ (capa > 0) o $M_0 = 2M$ (capa 0)

**Búsqueda**:

1. Fase de descenso: desde la capa más alta, busca greedy el nodo más cercano al query
2. Fase de expansión: en capa 0, expande con `ef_search` candidatos usando un beam search con heap

**Distancia utilizada**: Euclídea $d(\mathbf{a}, \mathbf{b}) = \|\mathbf{a} - \mathbf{b}\|_2$, que es monotónicamente equivalente a la similitud coseno para vectores normalizados:

$$\|\mathbf{a} - \mathbf{b}\|_2^2 = 2 - 2\cos(\mathbf{a}, \mathbf{b})$$

### 9.4 Embedding Bridge

El módulo `embedding_bridge` define el trait `EmbeddingBackend` para abstracción del modelo de embeddings. La implementación `MockEmbeddingBackend` genera vectores deterministas mediante FNV-1a hash + LCG, mientras que `OnnxEmbeddingBackend` está preparado para integración con ONNX Runtime en producción.

### 9.5 Complejidad

| Operación                    | Tiempo              | Espacio  |
| ---------------------------- | ------------------- | -------- |
| `dot_simd`                   | O(d/8)              | O(1)     |
| `linear_search` (N vectores) | O(N × d/8) paralelo | O(top_k) |
| HNSW `insert`                | O(M × log N × d/8)  | O(N × M) |
| HNSW `search`                | O(ef × log N × d/8) | O(ef)    |

---

## 10. Motor Central — rust_engine (FFI Bridge)

### 10.1 Arquitectura

`rust_engine` es el orquestador central que:

1. **Carga CSV**: Deserializa el catálogo en RAM usando `csv::ReaderBuilder` + `serde`
2. **Despacho de algoritmos**: Un `match` sobre `ffi::AlgoritmoType` pre-procesa la query según el algoritmo y ejecuta la búsqueda paralela con Rayon
3. **FFI Bridge**: Expone `SearchMaster` a C++ mediante `cxx::bridge` con namespace `ffi`
4. **Post-procesamiento**: Filtra por umbral (>0.1), ordena por score descendente, trunca a top-50

### 10.2 Pre-procesamiento Condicional

La query se pre-procesa de forma diferente según el algoritmo para evitar trabajo redundante dentro del loop paralelo:

- `SorensenDice`: Genera shingles una vez
- `Jaccard`: Tokeniza y hashea una vez
- `Phonetic`: Codifica con Double Metaphone una vez
- `Hamming` / `DamerauLevenshtein`: Sin pre-procesamiento especial

---

## 11. Frontend — Qt6/QML (The Omnibox)

### 11.1 Arquitectura

- **SearchModel** (C++): Hereda `QAbstractListModel`, posee `rust::Box<ffi::SearchMaster>` y expone `Q_INVOKABLE search()` a QML
- **Main.qml**: UI declarativa con Qt6 Quick, Kirigami para temas KDE, `MultiEffect` para blur acrylic
- **Debounce**: `Timer` de 50ms evita spamming del motor Rust durante escritura rápida
- **CMakeLists.txt**: Enlaza `librust_engine.a` estáticamente, configura `-march=native -O3`

### 11.2 Flujo de Datos

```
User types → QML TextField → debounceTimer (50ms) → SearchModel::search()
→ ffi::buscar(query, AlgoritmoType) → Rust Rayon par_iter → Vec<SearchResult>
→ QAbstractListModel::data() → QML delegate rendering
```
