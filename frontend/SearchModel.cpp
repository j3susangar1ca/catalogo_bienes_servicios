#include "SearchModel.h"
#include <QDebug>

SearchModel::SearchModel(QObject *parent) : QAbstractListModel(parent) {
    // 1. Instanciamos el motor de Rust (asignación de memoria HPC)
    m_searchMaster = ffi::new_search_master();
    
    // 2. CARGA EN RAM (WARM-UP)
    // Pasamos la ruta absoluta de tu archivo.
    // Pasamos la ruta relativa al ejecutable.
    QString csvPath = QCoreApplication::applicationDirPath() + "/catalogo.csv";
    
    qDebug() << "[HPC ENGINE] Iniciando carga de catálogo en 64GB RAM...";
    bool success = m_searchMaster->cargar_catalogo(csvPath.toStdString());
    
    if(success) {
        qDebug() << "[HPC ENGINE] Catálogo vectorizado y listo para The Omnibox.";
    } else {
        qWarning() << "[ERROR] No se pudo leer catalogo.csv. Verifica la ruta.";
    }
}

int SearchModel::activeAlgorithm() const { return m_activeAlgorithm; }

void SearchModel::setActiveAlgorithm(int algoIndex) {
    if (m_activeAlgorithm != algoIndex) {
        m_activeAlgorithm = algoIndex;
        emit algorithmChanged();
    }
}

void SearchModel::search(const QString &query) {
    // Si la búsqueda está vacía, limpiamos la lista visualmente
    if(query.trimmed().isEmpty()) {
        beginResetModel();
        m_results.clear();
        endResetModel();
        return;
    }

    // Le decimos a la UI que vamos a cambiar los datos (para las animaciones de QML)
    beginResetModel();
    
    // Mapeo del UI (0 al 4) a los Tipos de Rust
    ffi::AlgoritmoType rustAlgo;
    switch(m_activeAlgorithm) {
        case 0: rustAlgo = ffi::AlgoritmoType::Hamming; break;
        case 1: rustAlgo = ffi::AlgoritmoType::SorensenDice; break;
        case 2: rustAlgo = ffi::AlgoritmoType::Phonetic; break;
        case 3: rustAlgo = ffi::AlgoritmoType::DamerauLevenshtein; break;
        case 4: rustAlgo = ffi::AlgoritmoType::Jaccard; break;
        case 5: rustAlgo = ffi::AlgoritmoType::JaroWinkler; break;
        default: rustAlgo = ffi::AlgoritmoType::Hamming;
    }

    // DISPARO A RUST: Rayon usa los 8 núcleos de tu i7
    auto rustResults = m_searchMaster->buscar(query.toStdString(), rustAlgo);

    m_results.clear();
    for (const auto& res : rustResults) {
        m_results.push_back(res);
    }

    endResetModel();
}

QVariant SearchModel::data(const QModelIndex &index, int role) const {
    if (!index.isValid() || index.row() >= (int)m_results.size()) return QVariant();

    const auto &item = m_results[index.row()];
    switch (role) {
        case IdRole: return QString::fromStdString(std::string(item.id));
        case NombreRole: return QString::fromStdString(std::string(item.nombre));
        case ScoreRole: return item.score;
    }
    return QVariant();
}

int SearchModel::rowCount(const QModelIndex &parent) const {
    if (parent.isValid()) return 0;
    return m_results.size();
}

QHash<int, QByteArray> SearchModel::roleNames() const {
    QHash<int, QByteArray> roles;
    roles[IdRole] = "id";
    roles[NombreRole] = "nombre";
    roles[ScoreRole] = "score";
    return roles;
}
