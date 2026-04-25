cmake_minimum_required(VERSION 3.16)

project(Omnibox VERSION 0.1 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

# Buscar Qt 6
find_package(Qt6 REQUIRED COMPONENTS Quick QuickControls2 QuickLayouts QuickEffects)

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
    Qt6::QuickControls2
    Qt6::QuickLayouts
    Qt6::QuickEffects
    ${CMAKE_CURRENT_SOURCE_DIR}/../target/debug/librust_engine.a
    dl rt m pthread
)

# Ruta para encontrar lib.rs.h (generada por cxx en el target dir de Rust)
target_include_directories(Omnibox PRIVATE 
    ${CMAKE_CURRENT_BINARY_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../target/debug/build/rust_engine-32e7661fdbf06cf0/out/cxxbridge/include
)

# Nota: SearchModel.h incluye "lib.rs.h". 
# Necesitamos asegurarnos de que el compilador lo encuentre.
# CXX genera esto en la estructura de directorios del crate.
# Una forma común es que esté en target/cxxbridge/rust_engine/src/lib.rs.h
