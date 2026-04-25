#pragma once
#include <QAbstractListModel>
#include <QString>
#include <vector>
#include <QFutureWatcher>
#include "rust_engine/src/lib.rs.h" // El puente FFI generado por CXX

class SearchModel : public QAbstractListModel {
    Q_OBJECT
    // Propiedad que QML leerá y modificará cuando cambies de algoritmo (Chips)
    Q_PROPERTY(int activeAlgorithm READ activeAlgorithm WRITE setActiveAlgorithm NOTIFY algorithmChanged)

public:
    enum Roles {
        IdRole = Qt::UserRole + 1,
        NombreRole,
        ScoreRole
    };

    explicit SearchModel(QObject *parent = nullptr);

    // Métodos obligatorios de Qt para listas fluidas
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    QHash<int, QByteArray> roleNames() const override;

    // Métodos invocables desde QML
    Q_INVOKABLE void search(const QString &query);
    
    int activeAlgorithm() const;
    void setActiveAlgorithm(int algoIndex);

signals:
    void algorithmChanged();

private:
    // Puntero inteligente al motor de Rust
    rust::Box<ffi::SearchMaster> m_searchMaster;
    
    // Resultados cacheados para la UI
    std::vector<ffi::SearchResult> m_results;
    int m_activeAlgorithm = 0; // 0 = Hamming por defecto

    QFutureWatcher<rust::Vec<ffi::SearchResult>>* m_watcher = nullptr;
    bool m_searchInProgress = false;
};

