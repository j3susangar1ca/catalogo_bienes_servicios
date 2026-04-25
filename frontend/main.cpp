#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QDir>
#include <QFile>
#include <QDebug>
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
    // Estrategia: buscar QML relativo al binario, luego en QRC, luego fallar
    QStringList searchPaths = {
        QCoreApplication::applicationDirPath() + "/qml/Main.qml",
        QCoreApplication::applicationDirPath() + "/../frontend/Main.qml",
        QStringLiteral("qrc:/TheOmnibox/Main.qml")
    };

    QUrl url;
    for (const auto& path : searchPaths) {
        if (path.startsWith(QStringLiteral("qrc:"))) {
            url = QUrl(path);
            break;
        } else if (QFile::exists(path)) {
            url = QUrl::fromLocalFile(path);
            break;
        }
    }

    if (url.isEmpty()) {
        qCritical() << "Main.qml not found in any search path:" << searchPaths;
        return -1;
    }
    
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);

    engine.load(url);

    return app.exec();
}
