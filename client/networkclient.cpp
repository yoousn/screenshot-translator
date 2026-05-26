#include "networkclient.h"
#include <QHttpMultiPart>
#include <QHttpPart>
#include <QBuffer>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QUrlQuery>

NetworkClient::NetworkClient(QObject *parent) : QObject(parent) {
    manager = new QNetworkAccessManager(this);
}

void NetworkClient::translateImage(const QPixmap &pixmap, const ClientConfig &cfg, 
                                   std::function<void(bool success, const QPixmap &resPixmap)> callback) {
    QByteArray ba;
    QBuffer buffer(&ba);
    buffer.open(QIODevice::WriteOnly);
    pixmap.save(&buffer, "PNG");
    
    QHttpMultiPart *multiPart = new QHttpMultiPart(QHttpMultiPart::FormDataType);
    QHttpPart imagePart;
    imagePart.setHeader(QNetworkRequest::ContentTypeHeader, QVariant("image/png"));
    imagePart.setHeader(QNetworkRequest::ContentDispositionHeader, 
                        QVariant("form-data; name=\"image\"; filename=\"screenshot.png\""));
    imagePart.setBody(ba);
    multiPart->append(imagePart);
    
    // 确保 URL 格式正确，自动补足 http/https 协议前缀
    QString fullUrl = cfg.serverUrl;
    if (!fullUrl.startsWith("http://") && !fullUrl.startsWith("https://")) {
        fullUrl = "http://" + fullUrl;
    }
    QNetworkRequest request(QUrl(fullUrl + "/api/translate"));
    request.setRawHeader("X-API-Key", cfg.clientToken.toUtf8());
    
    QNetworkReply *reply = manager->post(request, multiPart);
    multiPart->setParent(reply); // 随请求一同释放
    
    connect(reply, &QNetworkReply::finished, [reply, callback]() {
        if (reply->error() == QNetworkReply::NoError) {
            QPixmap outPix;
            if (outPix.loadFromData(reply->readAll())) {
                callback(true, outPix);
            } else {
                callback(false, QPixmap());
            }
        } else {
            callback(false, QPixmap());
        }
        reply->deleteLater();
    });
}

void NetworkClient::ocrImage(const QPixmap &pixmap, const ClientConfig &cfg,
                             std::function<void(bool success, const QJsonArray &ocrResults)> callback) {
    QByteArray ba;
    QBuffer buffer(&ba);
    buffer.open(QIODevice::WriteOnly);
    pixmap.save(&buffer, "PNG");
    
    QHttpMultiPart *multiPart = new QHttpMultiPart(QHttpMultiPart::FormDataType);
    QHttpPart imagePart;
    imagePart.setHeader(QNetworkRequest::ContentTypeHeader, QVariant("image/png"));
    imagePart.setHeader(QNetworkRequest::ContentDispositionHeader, 
                        QVariant("form-data; name=\"image\"; filename=\"screenshot.png\""));
    imagePart.setBody(ba);
    multiPart->append(imagePart);
    
    QString fullUrl = cfg.serverUrl;
    if (!fullUrl.startsWith("http://") && !fullUrl.startsWith("https://")) {
        fullUrl = "http://" + fullUrl;
    }
    QNetworkRequest request(QUrl(fullUrl + "/api/ocr"));
    request.setRawHeader("X-API-Key", cfg.clientToken.toUtf8());
    
    QNetworkReply *reply = manager->post(request, multiPart);
    multiPart->setParent(reply);
    
    connect(reply, &QNetworkReply::finished, [reply, callback]() {
        if (reply->error() == QNetworkReply::NoError) {
            QJsonObject res = QJsonDocument::fromJson(reply->readAll()).object();
            if (res.value("status").toString() == "success") {
                callback(true, res.value("ocr").toArray());
            } else {
                callback(false, QJsonArray());
            }
        } else {
            callback(false, QJsonArray());
        }
        reply->deleteLater();
    });
}

void NetworkClient::testConfig(const ClientConfig &cfg, const QJsonObject &testPayload,
                               std::function<void(bool success, const QString &msg)> callback) {
    QString fullUrl = cfg.serverUrl;
    if (!fullUrl.startsWith("http://") && !fullUrl.startsWith("https://")) {
        fullUrl = "http://" + fullUrl;
    }
    QNetworkRequest request(QUrl(fullUrl + "/api/config/test"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    request.setRawHeader("X-API-Key", cfg.clientToken.toUtf8());
    
    QByteArray postData = QJsonDocument(testPayload).toJson();
    QNetworkReply *reply = manager->post(request, postData);
    
    connect(reply, &QNetworkReply::finished, [reply, callback]() {
        if (reply->error() == QNetworkReply::NoError) {
            QJsonObject res = QJsonDocument::fromJson(reply->readAll()).object();
            if (res.value("status").toString() == "success") {
                callback(true, res.value("result").toString());
            } else {
                callback(false, res.value("error").toString());
            }
        } else {
            callback(false, reply->errorString());
        }
        reply->deleteLater();
    });
}

void NetworkClient::fetchModels(const ClientConfig &cfg, const QString &baseUrl, const QString &apiKey,
                                std::function<void(bool success, const QStringList &models)> callback) {
    QString fullUrl = cfg.serverUrl;
    if (!fullUrl.startsWith("http://") && !fullUrl.startsWith("https://")) {
        fullUrl = "http://" + fullUrl;
    }
    QNetworkRequest request(QUrl(fullUrl + "/api/config/fetch_models"));
    request.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    request.setRawHeader("X-API-Key", cfg.clientToken.toUtf8());
    
    QJsonObject payload;
    payload["base_url"] = baseUrl;
    payload["api_key"] = apiKey;
    
    QByteArray postData = QJsonDocument(payload).toJson();
    QNetworkReply *reply = manager->post(request, postData);
    
    connect(reply, &QNetworkReply::finished, [reply, callback]() {
        if (reply->error() == QNetworkReply::NoError) {
            QJsonObject res = QJsonDocument::fromJson(reply->readAll()).object();
            if (res.value("status").toString() == "success") {
                QStringList mList;
                QJsonArray arr = res.value("models").toArray();
                for (auto val : arr) {
                    mList << val.toString();
                }
                callback(true, mList);
            } else {
                callback(false, QStringList());
            }
        } else {
            callback(false, QStringList());
        }
        reply->deleteLater();
    });
}
