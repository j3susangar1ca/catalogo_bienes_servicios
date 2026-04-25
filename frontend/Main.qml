import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami 2.20 as Kirigami
import QtQuick.Effects // Requerido para MultiEffect (Blur de Qt6)
import com.omnibox.search 1.0 // Modelo C++ de búsqueda

ApplicationWindow {
    id: window
    width: 850
    height: 700
    visible: true
    title: "The Omnibox"
    
    // 1. CONFIGURACIÓN DE VENTANA FLOTANTE KDE
    color: "transparent"
    flags: Qt.FramelessWindowHint | Qt.Window | Qt.WindowStaysOnTopHint // Flotante sobre todo

    // Paleta de colores para los algoritmos (Reacción visual)
    property var algoColors: [
        "#1d99f3", // Hamming - Azul KDE
        "#27ae60", // Sørensen - Verde Esmeralda
        "#9b59b6", // Metaphone - Púrpura
        "#f67400", // Damerau - Naranja
        "#da4453", // Jaccard - Rojo
        "#f39c12", // Jaro-Winkler - Ámbar
        "#34495e"  // Coseno - Azul Petróleo
    ]

    // El modelo C++ que conecta con RUST
    SearchModel { id: rustModel }

    // 2. DEBOUNCE TIMER: Evita spam al motor Rust mientras escribes rápido
    Timer {
        id: debounceTimer
        interval: 50 // 50ms de espera
        onTriggered: rustModel.search(searchInput.text)
    }

    // 3. SISTEMA DE ARRASTRE (Drag) para ventana sin bordes
    MouseArea {
        anchors.fill: parent
        property variant clickPos: "1,1"
        onPressed: (mouse) => clickPos = Qt.point(mouse.x, mouse.y)
        onPositionChanged: (mouse) => {
            if (pressed) {
                window.x += (mouse.x - clickPos.x);
                window.y += (mouse.y - clickPos.y);
            }
        }
    }

    // --- CAPA VISUAL DE FONDO (ACRYLIC/MICA BLUR) ---
    Rectangle {
        id: bgRect
        anchors.fill: parent
        radius: 16
        color: Kirigami.Theme.backgroundColor
        opacity: 0.85
        
        // Magia de Qt 6: Blur del escritorio real
        layer.enabled: true
        layer.effect: MultiEffect {
            blurEnabled: true
            blur: 0.6 // Intensidad del desenfoque
            blurMax: 32
            blurMultiplier: 1.0
        }

        // Borde sutil luminoso
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: "transparent"
            border.width: 1
            border.color: Qt.rgba(255, 255, 255, 0.1)
        }
    }

    // --- CONTENIDO PRINCIPAL ---
    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 24
        spacing: 20

        // Botón de cerrar minimalista (Arriba a la derecha)
        Button {
            Layout.alignment: Qt.AlignRight
            text: "✕"
            flat: true
            onClicked: Qt.quit()
            background: Rectangle { color: "transparent" }
            contentItem: Text { 
                text: parent.text; color: Kirigami.Theme.textColor; 
                font.pixelSize: 16; opacity: parent.hovered ? 1 : 0.5 
                Behavior on opacity { NumberAnimation { duration: 150 } }
            }
        }

        // 1. BARRA BUSCADORA GIGANTE
        TextField {
            id: searchInput
            Layout.fillWidth: true
            font.pixelSize: 28
            font.weight: Font.Light
            placeholderText: "Buscar en el catálogo..."
            placeholderTextColor: Qt.rgba(255, 255, 255, 0.3)
            color: Kirigami.Theme.textColor
            verticalAlignment: Text.AlignVCenter
            
            // Cambio de color de acento según el algoritmo
            selectionColor: algoColors[rustModel.activeAlgorithm]
            selectedTextColor: "#FFFFFF"

            background: Rectangle {
                color: "transparent"
                border.width: 0
                // Línea inferior animada
                Rectangle {
                    anchors.bottom: parent.bottom
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: searchInput.activeFocus ? parent.width : 0
                    height: 3
                    radius: 1.5
                    color: algoColors[rustModel.activeAlgorithm]
                    
                    Behavior on width { 
                        NumberAnimation { duration: 300; easing.type: Easing.OutCubic } 
                    }
                    Behavior on color { ColorAnimation { duration: 250 } }
                }
            }
            
            // Dispara el debounce en lugar de buscar directamente
            onTextChanged: debounceTimer.restart()
        }

        // 2. CHIPS DE ALGORITMOS (Reactivos y Mutantes)
        RowLayout {
            spacing: 12
            Repeater {
                model: ["Hamming HPC", "Sørensen-Dice", "Metaphone", "Damerau-Lev", "Jaccard", "Jaro-Winkler", "Semántica (Coseno)"]
                
                AbstractButton {
                    text: modelData
                    Layout.alignment: Qt.AlignVCenter
                    
                    property bool isActive: rustModel.activeAlgorithm === index
                    
                    // Efecto Hover y Click
                    hoverEnabled: true
                    
                    onClicked: {
                        rustModel.activeAlgorithm = index
                        debounceTimer.restart() // Re-busca con nuevo algoritmo
                        
                        // Mutar el color de acento global de Kirigami
                        Kirigami.Theme.highlightColor = algoColors[index]
                    }

                    background: Rectangle {
                        radius: 16
                        height: 36
                        // Animación de color suave
                        color: isActive ? algoColors[rustModel.activeAlgorithm] : Qt.rgba(255,255,255,0.05)
                        opacity: parent.hovered && !isActive ? 1.0 : (isActive ? 1.0 : 0.7)
                        
                        Behavior on color { ColorAnimation { duration: 250 } }
                        Behavior on opacity { NumberAnimation { duration: 150 } }
                        
                        // Escala al hacer click (Micro-interacción)
                        scale: parent.pressed ? 0.95 : 1.0
                        Behavior on scale { NumberAnimation { duration: 100; easing.type: Easing.OutCubic } }
                    }

                    contentItem: Text {
                        text: parent.text
                        font.pixelSize: 14
                        font.bold: parent.isActive
                        color: parent.isActive ? "#FFFFFF" : Kirigami.Theme.textColor
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                        
                        Behavior on color { ColorAnimation { duration: 250 } }
                    }
                }
            }
        }

        // 3. LISTA DE RESULTADOS FLUIDA (Staggered Cascade)
        ListView {
            id: resultList
            Layout.fillWidth: true
            Layout.fillHeight: true
            model: rustModel
            clip: true
            spacing: 12

            // Transición escalonada en cascada
            add: Transition {
                SequentialAnimation {
                    NumberAnimation { property: "opacity"; from: 0; to: 1; duration: 300; easing.type: Easing.InOutQuad }
                    NumberAnimation { property: "y"; from: 20; to: 0; duration: 300; easing.type: Easing.OutCubic }
                }
            }
            
            displaced: Transition {
                NumberAnimation { properties: "x,y"; duration: 200; easing.type: Easing.OutCubic }
            }

            delegate: Item {
                width: ListView.view.width
                height: 80
                
                // Tarjeta del resultado
                Rectangle {
                    id: card
                    anchors.fill: parent
                    radius: 12
                    color: Kirigami.Theme.alternateBackgroundColor
                    opacity: 0.9
                    
                    // Hover Effect
                    property bool isHovered: mouseArea.containsMouse
                    Behavior on scale { NumberAnimation { duration: 150 } }
                    scale: isHovered ? 1.02 : 1.0
                    
                    Rectangle {
                        anchors.fill: parent
                        radius: parent.radius
                        color: "transparent"
                        border.width: 1
                        border.color: isHovered ? algoColors[rustModel.activeAlgorithm] : "transparent"
                        Behavior on border.color { ColorAnimation { duration: 150 } }
                    }

                    MouseArea { id: mouseArea; anchors.fill: parent; hoverEnabled: true }

                    RowLayout {
                        anchors.fill: parent
                        anchors.margins: 16
                        
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 4
                            
                            Text { 
                                text: model.nombre; 
                                font.pixelSize: 18; 
                                font.bold: true; 
                                color: Kirigami.Theme.textColor;
                                // Resaltado si está hovered
                                opacity: card.isHovered ? 1 : 0.9
                            }
                            Text { 
                                text: "SKU: " + model.id; 
                                font.pixelSize: 13; 
                                color: Kirigami.Theme.disabledTextColor; 
                                font.family: "JetBrains Mono, Fira Code, monospace" 
                            }
                        }

                        // Score Bar Fluida
                        ColumnLayout {
                            spacing: 2
                            Text {
                                text: (model.score * 100).toFixed(1) + "%"
                                font.pixelSize: 16
                                font.bold: true
                                color: algoColors[rustModel.activeAlgorithm]
                                Layout.alignment: Qt.AlignRight
                                Behavior on color { ColorAnimation { duration: 250 } }
                            }
                            Rectangle {
                                Layout.preferredWidth: 120
                                Layout.preferredHeight: 6
                                radius: 3
                                color: Qt.rgba(255,255,255,0.1)
                                
                                Rectangle {
                                    width: parent.width * model.score
                                    height: parent.height
                                    radius: parent.radius
                                    color: algoColors[rustModel.activeAlgorithm]
                                    
                                    // Animación de llenado (Elastic)
                                    Behavior on width { 
                                        NumberAnimation { duration: 600; easing.type: Easing.OutCubic } 
                                    }
                                    Behavior on color { ColorAnimation { duration: 250 } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
