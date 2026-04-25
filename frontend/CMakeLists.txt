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

# Buscar dinámicamente el directorio de cxxbridge generado por Cargo
execute_process(
    COMMAND find ${CMAKE_CURRENT_SOURCE_DIR}/../target -path "*/rust_engine-*/out/cxxbridge/include"
    COMMAND head -n 1
    OUTPUT_VARIABLE CXXBRIDGE_INCLUDE_DIR
    OUTPUT_STRIP_TRAILING_WHITESPACE
)

if(NOT CXXBRIDGE_INCLUDE_DIR)
    message(FATAL_ERROR 
        "cxxbridge include dir not found. Run 'cargo build -p rust_engine' first."
    )
endif()

message(STATUS "cxxbridge include: ${CXXBRIDGE_INCLUDE_DIR}")

# Buscar dinámicamente la librería estática de Rust
find_file(RUST_ENGINE_LIB_DEBUG
    NAMES librust_engine.a
    PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../target/debug
    NO_DEFAULT_PATH
)
find_file(RUST_ENGINE_LIB_RELEASE
    NAMES librust_engine.a
    PATHS ${CMAKE_CURRENT_SOURCE_DIR}/../target/release
    NO_DEFAULT_PATH
)

set(RUST_ENGINE_LIB "${RUST_ENGINE_LIB_RELEASE}")
if(NOT RUST_ENGINE_LIB)
    set(RUST_ENGINE_LIB "${RUST_ENGINE_LIB_DEBUG}")
endif()

if(NOT RUST_ENGINE_LIB)
    message(FATAL_ERROR 
        "librust_engine.a not found. Run 'cargo build -p rust_engine' first."
    )
endif()

message(STATUS "Rust engine lib: ${RUST_ENGINE_LIB}")

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

target_include_directories(OmniboxLauncher PRIVATE 
    ${CMAKE_CURRENT_BINARY_DIR}
    ${CXXBRIDGE_INCLUDE_DIR}
)

# Optimizaciones para CPU i7-9700
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(OmniboxLauncher PRIVATE -march=native -O3 -pipe)
endif()
