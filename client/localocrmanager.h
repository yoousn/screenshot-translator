#pragma once
#include <QObject>
#include <QJsonArray>
#include <QPixmap>
#include <QProcess>
#include <QTimer>
#include <functional>
#include "config.h"

class LocalOcrManager : public QObject {
    Q_OBJECT
public:
    explicit LocalOcrManager(QObject *parent = nullptr);
    ~LocalOcrManager() override;

    void ocrImage(const QPixmap &pixmap, const ClientConfig &cfg,
                  std::function<void(bool success, const QJsonArray &ocrResults, const QString &error)> callback);

private:
    void finishRequest(bool success, const QJsonArray &ocrResults, const QString &error);
    QJsonArray normalizeResult(const QByteArray &data, bool *ok) const;

    QProcess *process = nullptr;
    QTimer *timeoutTimer = nullptr;
    std::function<void(bool success, const QJsonArray &ocrResults, const QString &error)> currentCallback;
    QByteArray stdoutBuffer;
    QByteArray stderrBuffer;
    QString currentImagePath;
    bool requestInFlight = false;
};
