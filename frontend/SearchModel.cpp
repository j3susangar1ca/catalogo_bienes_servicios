#include "SearchModel.h"
#include <QStandardPaths>
#include <QDir>
#include <QDebug>
#include <QCoreApplication>
#include <QFile>
#include <QFuture>
#include <QtConcurrent/QtConcurrent>

SearchModel::SearchModel(QObject *parent) 
    : QAbstractListModel(parent), 
      m_searchMaster(ffi::new_search_master()) 
{
    // Búsqueda del CSV en múltiples ubicaciones razonables
    QString csvPath = qEnvironmentVariable("CATALOGO_CSV_PATH");
    
    if (csvPath.isEmpty()) {
        QStringList searchPaths = {
            QCoreApplication::applicationDirPath() + "/catalogo.csv",
            QCoreApplication::applicationDirPath() + "/../catalogo.csv",
            QCoreApplication::applicationDirPath() + "/data/catalogo.csv",
            QStandardPaths::locate(QStandardPaths::AppDataLocation, "catalogo.csv"),
        };
        
        for (const auto& path : searchPaths) {
            if (QFile::exists(path)) {
                csvPath = path;
                break;
            }
        }
    }
    
    if (csvPath.isEmpty()) {
        qCritical() << "[ERROR] catalogo.csv no encontrado. "
                    << "Defina la variable de entorno CATALOGO_CSV_PATH "
                    << "o coloque catalogo.csv junto al ejecutable.";
        return;  // La UI se muestra pero las búsquedas retornan vacío
    }
    
    qDebug() << "[HPC ENGINE] Cargando catálogo desde:" << csvPath;
    qDebug() << "[HPC ENGINE] Iniciando carga de catálogo en 64GB RAM...";
    bool success = m_searchMaster->cargar_catalogo(csvPath.toStdString());
    
    if(success) {
        qDebug() << "[HPC ENGINE] Catálogo vectorizado y listo para The Omnibox.";
    } else {
        qWarning() << "[ERROR] Falló la carga de" << csvPath;
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
    if(query.trimmed().isEmpty()) {
        beginResetModel();
        m_results.clear();
        endResetModel();
        return;
    }

    if (m_searchInProgress) {
        return;  // Evitar búsquedas superpuestas
    }

    m_searchInProgress = true;

    ffi::AlgoritmoType rustAlgo;
    switch(m_activeAlgorithm) {
        case 0: rustAlgo = ffi::AlgoritmoType::Hamming; break;
        case 1: rustAlgo = ffi::AlgoritmoType::SorensenDice; break;
        case 2: rustAlgo = ffi::AlgoritmoType::Phonetic; break;
        case 3: rustAlgo = ffi::AlgoritmoType::DamerauLevenshtein; break;
        case 4: rustAlgo = ffi::AlgoritmoType::Jaccard; break;
        case 5: rustAlgo = ffi::AlgoritmoType::JaroWinkler; break;
        case 6: rustAlgo = ffi::AlgoritmoType::Cosine; break;
        default: rustAlgo = ffi::AlgoritmoType::Hamming;
    }

    auto future = QtConcurrent::run([this, query, rustAlgo]() {
        return m_searchMaster->buscar(query.toStdString(), rustAlgo);
    });

    if (!m_watcher) {
        m_watcher = new QFutureWatcher<rust::Vec<ffi::SearchResult>>(this);
        connect(m_watcher, &QFutureWatcherBase::finished, this, [this]() {
            beginResetModel();
            m_results.clear();
            auto rustResults = m_watcher->result();
            for (const auto& res : rustResults) {
                m_results.push_back(res);
            }
            m_searchInProgress = false;
            endResetModel();
        });
    }

    m_watcher->setFuture(future);
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
