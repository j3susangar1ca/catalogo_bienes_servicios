cmake_minimum_required(VERSION 3.25)
project(TheOmnibox)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

# Buscar Qt 6 y Kirigami
find_package(Qt6 REQUIRED COMPONENTS Core Gui Quick QuickControls2 QuickLayouts QuickEffects Widgets)
find_package(KF6Kirigami REQUIRED)

# El ejecutable
qt_add_executable(OmniboxLauncher
    main.cpp
    SearchModel.cpp
    SearchModel.h
)

# Módulo QML
qt_add_qml_module(OmniboxLauncher
    URI TheOmnibox
    VERSION 1.0
    QML_FILES Main.qml
)

# Enlace de librerías
# Buscar dinámicamente la librería de Rust (debug o release)
find_file(RUST_ENGINE_LIB
    NAMES librust_engine.a
    PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../target/debug
          ${CMAKE_CURRENT_SOURCE_DIR}/../target/release
    NO_DEFAULT_PATH
)

if(NOT RUST_ENGINE_LIB)
    message(FATAL_ERROR "Could not find librust_engine.a. Run `cargo build` first.")
endif()

target_link_libraries(OmniboxLauncher PRIVATE
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::QuickLayouts
    Qt6::QuickEffects
    Qt6::Widgets
    KF6::Kirigami
    ${RUST_ENGINE_LIB}
    dl rt m pthread
)

# Ruta para encontrar lib.rs.h y cxxbridge headers
# Buscar dinámicamente el directorio de cxxbridge include
execute_process(
    COMMAND find ${CMAKE_CURRENT_SOURCE_DIR}/../target -type d -path "*/rust_engine-*/out/cxxbridge/include"
    OUTPUT_VARIABLE CXXBRIDGE_INCLUDE_DIR
    OUTPUT_STRIP_TRAILING_WHITESPACE
)
string(REPLACE "\n" ";" CXXBRIDGE_INCLUDE_DIR "${CXXBRIDGE_INCLUDE_DIR}")

if(NOT CXXBRIDGE_INCLUDE_DIR)
    # Fallback: usar el directorio include estándar de rust_engine
    set(CXXBRIDGE_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../rust_engine/include)
endif()

target_include_directories(OmniboxLauncher PRIVATE 
    ${CMAKE_CURRENT_BINARY_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../rust_engine/include
    ${CXXBRIDGE_INCLUDE_DIR}
)

# Optimizaciones para CPU i7-9700
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(OmniboxLauncher PRIVATE -march=native -O3 -pipe)
endif()
