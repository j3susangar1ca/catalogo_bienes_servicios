cmake_minimum_required(VERSION 3.16)

project(Omnibox VERSION 0.1 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Buscar Qt 6
find_package(Qt6 REQUIRED COMPONENTS Quick QuickControls2 QuickLayouts QuickEffects)
# Buscar Kirigami (KDE)
find_package(KF6 REQUIRED COMPONENTS Kirigami)

# Integración con Rust vía Corrosion
include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.5.0
)
FetchContent_MakeAvailable(Corrosion)

# Importar el crate de Rust
corrosion_import_crate(MANIFEST_PATH ../rust_engine/Cargo.toml)

# El ejecutable
qt_add_executable(Omnibox
    main.cpp
    SearchModel.cpp
    SearchModel.h
)

# Módulo QML
qt_add_qml_module(Omnibox
    URI TheOmnibox
    VERSION 1.0
    QML_FILES Main.qml
)

# Enlace de librerías
target_link_libraries(Omnibox PRIVATE
    Qt6::Quick
    Qt6::Controls2
    Qt6::Layouts
    Qt6::Effects
    KF6::Kirigami
    rust_engine-staticlib
)

# Ruta para encontrar lib.rs.h (generada por cxx en el build dir de Rust)
# Corrosion pone los artefactos de build en directorios específicos.
# Por simplicidad, añadimos la ruta donde cxx_build suele generar las cabeceras.
target_include_directories(Omnibox PRIVATE 
    ${CMAKE_CURRENT_BINARY_DIR}
    # La ruta exacta puede variar, pero cxx_build genera cabeceras en OUT_DIR.
    # Con Corrosion, podemos obtener la ruta de los artefactos.
)

# Nota: SearchModel.h incluye "lib.rs.h". 
# Necesitamos asegurarnos de que el compilador lo encuentre.
# CXX genera esto en la estructura de directorios del crate.
# Una forma común es que esté en target/cxxbridge/rust_engine/src/lib.rs.h
