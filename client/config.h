#pragma once
#include <QString>
#include <QJsonObject>

struct ClientConfig {
    QString serverUrl = "https://ocr.yousn.me";
    QString clientToken = "ysn-screenshot-translator-token-666";
    QString channel = "new-api";
    QString newApiBase = "api.yousn.me";
    QString newApiKey = "sk-88AqJeSQhfrmVTDcSAOTZDb6NqEbG3X8C3na3WqolNdasdpb";
    QString newApiModel = "gemini-1.5-flash";
    QString baiduAppId = "";
    QString baiduSecretKey = "";

    void load();
    void save();
};
