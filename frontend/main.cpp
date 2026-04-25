#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include "SearchModel.h"

int main(int argc, char *argv[])
{
    QGuiApplication app(argc, argv);

    app.setOrganizationName("TheOmnibox");
    app.setApplicationName("Omnibox");

    qmlRegisterType<SearchModel>("TheOmnibox", 1, 0, "SearchModel");

    QQmlApplicationEngine engine;
    
    // Registrar el modelo para que esté disponible en QML si se prefiere una instancia global,
    // pero en el QML proporcionado se instancia como SearchModel { id: rustModel }.
    // qmlRegisterType arriba es suficiente.

    const QUrl url(u"qrc:/qt/qml/Main.qml"_qs);
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreated,
                     &app, [url](QObject *obj, const QUrl &objUrl) {
        if (!obj && url == objUrl)
            QCoreApplication::exit(-1);
    }, Qt::QueuedConnection);
    
    engine.load(url);

    return app.exec();
}
