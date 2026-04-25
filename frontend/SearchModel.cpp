#include "SearchModel.h"

SearchModel::SearchModel(QObject *parent)
    : QAbstractListModel(parent)
{
}

int SearchModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid())
        return 0;
    return m_results.size();
}

QVariant SearchModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() >= (int)m_results.size())
        return QVariant();

    const auto &result = m_results[index.row()];

    switch (role) {
    case IdRole:
        return QString::fromStdString(std::string(result.id));
    case NombreRole:
        return QString::fromStdString(std::string(result.nombre));
    case ScoreRole:
        return result.score;
    default:
        return QVariant();
    }
}

QHash<int, QByteArray> SearchModel::roleNames() const
{
    QHash<int, QByteArray> roles;
    roles[IdRole] = "id";
    roles[NombreRole] = "nombre";
    roles[ScoreRole] = "score";
    return roles;
}

void SearchModel::setActiveAlgorithm(int algorithm)
{
    if (m_activeAlgorithm == algorithm)
        return;

    m_activeAlgorithm = algorithm;
    emit activeAlgorithmChanged();
    
    // Re-buscar si hay una consulta previa
    if (!m_lastQuery.isEmpty()) {
        search(m_lastQuery);
    }
}

void SearchModel::search(const QString &query)
{
    m_lastQuery = query;
    std::string q = query.toStdString();
    rust::String rust_query(q);

    beginResetModel();
    m_results = search_in_rust(rust_query, m_activeAlgorithm);
    endResetModel();
}
