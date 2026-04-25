#ifndef SEARCHMODEL_H
#define SEARCHMODEL_H

#include <QAbstractListModel>
#include <QString>
#include <QVector>

// Cabecera generada por cxx
#include "rust_engine/src/lib.rs.h"

class SearchModel : public QAbstractListModel
{
    Q_OBJECT
    Q_PROPERTY(int activeAlgorithm READ activeAlgorithm WRITE setActiveAlgorithm NOTIFY algorithmChanged)

public:
    enum Roles {
        IdRole = Qt::UserRole + 1,
        NombreRole,
        ScoreRole
    };

    explicit SearchModel(QObject *parent = nullptr);

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    QHash<int, QByteArray> roleNames() const override;

    int activeAlgorithm() const { return m_activeAlgorithm; }
    void setActiveAlgorithm(int algorithm);

    Q_INVOKABLE void search(const QString &query);

signals:
    void algorithmChanged();

private:
    int m_activeAlgorithm = 0;
    rust::Box<SearchMaster> m_searchMaster;
    rust::Vec<SearchResult> m_results;
    QString m_lastQuery;
};

#endif // SEARCHMODEL_H
