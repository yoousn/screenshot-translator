#include "config.h"
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QDir>

void ClientConfig::load() {
    // 保存到用户文档或应用数据目录，确保可写
    QString path = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(path);
    QFile file(path + "/config.json");
    
    if (!file.open(QIODevice::ReadOnly)) return;
    QByteArray data = file.readAll();
    QJsonDocument doc = QJsonDocument::fromJson(data);
    if (doc.isNull()) return;
    QJsonObject obj = doc.object();
    
    serverUrl = obj.value("serverUrl").toString(serverUrl);
    clientToken = obj.value("clientToken").toString(clientToken);
    channel = obj.value("channel").toString(channel);
    newApiBase = obj.value("newApiBase").toString(newApiBase);
    newApiKey = obj.value("newApiKey").toString(newApiKey);
    newApiModel = obj.value("newApiModel").toString(newApiModel);
    baiduAppId = obj.value("baiduAppId").toString(baiduAppId);
    baiduSecretKey = obj.value("baiduSecretKey").toString(baiduSecretKey);
    useLocalOcr = obj.value("useLocalOcr").toBool(useLocalOcr);
    localOcrExecutablePath = obj.value("localOcrExecutablePath").toString(localOcrExecutablePath);
    localOcrTimeoutMs = obj.value("localOcrTimeoutMs").toInt(localOcrTimeoutMs);
    fallbackToRemoteOcr = obj.value("fallbackToRemoteOcr").toBool(fallbackToRemoteOcr);
}

void ClientConfig::save() {
    QString path = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(path);
    QFile file(path + "/config.json");
    
    if (!file.open(QIODevice::WriteOnly)) return;
    QJsonObject obj;
    obj["serverUrl"] = serverUrl;
    obj["clientToken"] = clientToken;
    obj["channel"] = channel;
    obj["newApiBase"] = newApiBase;
    obj["newApiKey"] = newApiKey;
    obj["newApiModel"] = newApiModel;
    obj["baiduAppId"] = baiduAppId;
    obj["baiduSecretKey"] = baiduSecretKey;
    obj["useLocalOcr"] = useLocalOcr;
    obj["localOcrExecutablePath"] = localOcrExecutablePath;
    obj["localOcrTimeoutMs"] = localOcrTimeoutMs;
    obj["fallbackToRemoteOcr"] = fallbackToRemoteOcr;
    file.write(QJsonDocument(obj).toJson(QJsonDocument::Indented));
}
