cmake_minimum_required(VERSION 3.25)
project(TheOmnibox)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

# Buscar Qt 6
find_package(Qt6 REQUIRED COMPONENTS Core Gui Quick QuickControls2 QuickLayouts QuickEffects Widgets)
# Kirigami se carga en tiempo de ejecución en QML, no lo enlazamos aquí para evitar errores de búsqueda de config

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
target_link_libraries(OmniboxLauncher PRIVATE
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::QuickLayouts
    Qt6::QuickEffects
    Qt6::Widgets
    ${CMAKE_CURRENT_SOURCE_DIR}/../target/debug/librust_engine.a
    dl rt m pthread
)

# Ruta para encontrar lib.rs.h (generada por cxx en el target dir de Rust)
target_include_directories(OmniboxLauncher PRIVATE 
    ${CMAKE_CURRENT_BINARY_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../target/debug/build/rust_engine-32e7661fdbf06cf0/out/cxxbridge/include
)

# Optimizaciones para CPU i7-9700
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    target_compile_options(OmniboxLauncher PRIVATE -march=native -O3 -pipe)
endif()
