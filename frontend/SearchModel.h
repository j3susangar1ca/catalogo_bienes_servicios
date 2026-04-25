#ifndef SEARCHMODEL_H
#define SEARCHMODEL_H

#include <QAbstractListModel>
#include <QString>
#include <QVector>

// Definición manual de la estructura para que C++ la conozca antes de que cxx genere la cabecera
// O mejor, incluimos la cabecera que generará cxx.
#include "lib.rs.h"

class SearchModel : public QAbstractListModel
{
    Q_OBJECT
    Q_PROPERTY(int activeAlgorithm READ activeAlgorithm WRITE setActiveAlgorithm NOTIFY activeAlgorithmChanged)

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
    void activeAlgorithmChanged();

private:
    int m_activeAlgorithm = 0;
    rust::Vec<ffi::SearchResult> m_results;
    QString m_lastQuery;
};

#endif // SEARCHMODEL_H
