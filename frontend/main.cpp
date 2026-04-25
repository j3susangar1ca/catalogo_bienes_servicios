#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QDir>
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
    // Ruta relativa al ejecutable para mayor portabilidad
    QString qmlPath = QDir::cleanPath(QCoreApplication::applicationDirPath() + "/Main.qml");
    const QUrl url = QUrl::fromLocalFile(qmlPath);
    
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);

    engine.load(url);

    return app.exec();
}
