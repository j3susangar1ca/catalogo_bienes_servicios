#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include "SearchModel.h"

int main(int argc, char *argv[]) {
    // Optimizaciones nativas para Wayland y monitores de alta tasa de refresco
    qputenv("QT_QPA_PLATFORM", "wayland;xcb");
    
    QGuiApplication app(argc, argv);
    app.setOrganizationName("EliteEngineering");
    app.setApplicationName("TheOmnibox");

    // 1. Registramos tu clase C++ para que QML la reconozca como un componente visual
    qmlRegisterType<SearchModel>("com.omnibox.search", 1, 0, "SearchModel");

    QQmlApplicationEngine engine;

    // 2. Cargamos la interfaz QML
    // Nota: Ajusta esta ruta si tu Main.qml está en otro lado. 
    // Usamos ruta absoluta temporal para garantizar que funcione al primer intento.
    // Usamos ruta relativa al ejecutable para mayor portabilidad.
    const QUrl url = QUrl::fromLocalFile(QCoreApplication::applicationDirPath() + "/Main.qml");
    
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);

    engine.load(url);

    return app.exec();
}
