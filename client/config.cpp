#include "config.h"
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QDir>
#include <QCoreApplication>

void ClientConfig::load() {
    // 保存到用户文档或应用数据目录，确保可写
    QString path = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    QDir().mkpath(path);
    QFile file(path + "/config.json");
    
    QString releasePath = QDir::cleanPath(QCoreApplication::applicationDirPath() + "/ocr/PaddleOCR-json_v1.4.1/PaddleOCR-json.exe");
    QString devPath = QDir::cleanPath(QCoreApplication::applicationDirPath() + "/../server/ocr/PaddleOCR-json_v1.4.1/PaddleOCR-json.exe");
    
    QString defaultPath = releasePath;
    if (!QFile::exists(releasePath) && QFile::exists(devPath)) {
        defaultPath = devPath;
    }
    
    if (!file.open(QIODevice::ReadOnly)) {
        localOcrExecutablePath = defaultPath;
        return;
    }
    QByteArray data = file.readAll();
    QJsonDocument doc = QJsonDocument::fromJson(data);
    if (doc.isNull()) {
        localOcrExecutablePath = defaultPath;
        return;
    }
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
    
    localOcrExecutablePath = obj.value("localOcrExecutablePath").toString();
    if (localOcrExecutablePath.isEmpty() || !QFile::exists(localOcrExecutablePath)) {
        localOcrExecutablePath = defaultPath;
    }
    
    localOcrTimeoutMs = obj.value("localOcrTimeoutMs").toInt(localOcrTimeoutMs);
    fallbackToRemoteOcr = obj.value("fallbackToRemoteOcr").toBool(fallbackToRemoteOcr);
    hotkey = obj.value("hotkey").toString(hotkey);
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
    obj["hotkey"] = hotkey;
    file.write(QJsonDocument(obj).toJson(QJsonDocument::Indented));
}
