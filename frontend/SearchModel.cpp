#include "SearchModel.h"
#include <QDebug>

SearchModel::SearchModel(QObject *parent) 
    : QAbstractListModel(parent)
    , m_searchMaster(new_search_master()) 
{
    // Carga inicial del catálogo (esto usará tus 64GB de RAM)
    // Nota: Asegúrate de que esta ruta sea válida o cámbiala según sea necesario.
    m_searchMaster->cargar_catalogo("/home/jesuslangarica/catalogo_bienes_servicios/backend/datos/catalogo_real.csv");
}

int SearchModel::rowCount(const QModelIndex &parent) const {
    if (parent.isValid()) return 0;
    return m_results.size();
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

QHash<int, QByteArray> SearchModel::roleNames() const {
    QHash<int, QByteArray> roles;
    roles[IdRole] = "id";
    roles[NombreRole] = "nombre";
    roles[ScoreRole] = "score";
    return roles;
}

void SearchModel::setActiveAlgorithm(int algoIndex) {
    if (m_activeAlgorithm != algoIndex) {
        m_activeAlgorithm = algoIndex;
        emit algorithmChanged();
        
        if (!m_lastQuery.isEmpty()) {
            search(m_lastQuery);
        }
    }
}

void SearchModel::search(const QString &query) {
    m_lastQuery = query;
    
    // 1. Limpiamos resultados previos con señales de inicio/fin para que QML anime
    beginResetModel();
    
    // 2. Mapeamos el index de la UI al enum de Rust
    AlgoritmoType rustAlgo;
    switch(m_activeAlgorithm) {
        case 0: rustAlgo = AlgoritmoType::Hamming; break;
        case 1: rustAlgo = AlgoritmoType::SorensenDice; break;
        case 2: rustAlgo = AlgoritmoType::Phonetic; break;
        case 3: rustAlgo = AlgoritmoType::DamerauLevenshtein; break;
        case 4: rustAlgo = AlgoritmoType::Jaccard; break;
        default: rustAlgo = AlgoritmoType::Hamming;
    }

    // 3. LLAMADA DE ALTO RENDIMIENTO A RUST
    rust::String rust_query(query.toStdString());
    auto rustResults = m_searchMaster->buscar(rust_query, rustAlgo);

    // 4. Actualizamos el vector interno
    m_results = std::move(rustResults);

    endResetModel();
}
