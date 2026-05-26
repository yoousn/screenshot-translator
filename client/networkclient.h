#pragma once
#include <QObject>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QPixmap>
#include <functional>
#include "config.h"

class NetworkClient : public QObject {
    Q_OBJECT
public:
    explicit NetworkClient(QObject *parent = nullptr);
    
    // 发起翻译截图
    void translateImage(const QPixmap &pixmap, const ClientConfig &cfg, 
                        std::function<void(bool success, const QPixmap &resPixmap)> callback);

    // 测试并保存配置
    void testConfig(const ClientConfig &cfg, const QJsonObject &testPayload,
                    std::function<void(bool success, const QString &msg)> callback);

    // 动态拉取大模型列表
    void fetchModels(const ClientConfig &cfg, const QString &baseUrl, const QString &apiKey,
                     std::function<void(bool success, const QStringList &models)> callback);

private:
    QNetworkAccessManager *manager;
};
