#include "localocrmanager.h"
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryFile>

LocalOcrManager::LocalOcrManager(QObject *parent) : QObject(parent) {
    process = new QProcess(this);
    timeoutTimer = new QTimer(this);
    timeoutTimer->setSingleShot(true);

    connect(timeoutTimer, &QTimer::timeout, this, [this]() {
        if (process->state() != QProcess::NotRunning) {
            process->kill();
        }
        finishRequest(false, QJsonArray(), "本地 OCR 超时");
    });

    connect(process, &QProcess::readyReadStandardOutput, this, [this]() {
        stdoutBuffer.append(process->readAllStandardOutput());
    });

    connect(process, &QProcess::readyReadStandardError, this, [this]() {
        stderrBuffer.append(process->readAllStandardError());
    });

    connect(process, qOverload<int, QProcess::ExitStatus>(&QProcess::finished), this,
            [this](int exitCode, QProcess::ExitStatus exitStatus) {
        if (!requestInFlight) return;
        if (exitStatus != QProcess::NormalExit || exitCode != 0) {
            finishRequest(false, QJsonArray(), QString::fromUtf8(stderrBuffer).trimmed());
            return;
        }
        bool ok = false;
        QJsonArray results = normalizeResult(stdoutBuffer, &ok);
        finishRequest(ok, results, ok ? QString() : "本地 OCR 返回格式异常");
    });

    connect(process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (!requestInFlight) return;
        Q_UNUSED(error);
        finishRequest(false, QJsonArray(), process->errorString());
    });
}

LocalOcrManager::~LocalOcrManager() {
    if (process && process->state() != QProcess::NotRunning) {
        process->kill();
        process->waitForFinished(1000);
    }
    if (!currentImagePath.isEmpty()) {
        QFile::remove(currentImagePath);
    }
}

void LocalOcrManager::ocrImage(const QPixmap &pixmap, const ClientConfig &cfg,
                               std::function<void(bool, const QJsonArray &, const QString &)> callback) {
    if (requestInFlight) {
        callback(false, QJsonArray(), "本地 OCR 正忙");
        return;
    }

    QFileInfo executable(cfg.localOcrExecutablePath);
    if (!executable.exists() || !executable.isFile() || !executable.isExecutable() || executable.isRelative()) {
        callback(false, QJsonArray(), "本地 OCR 引擎路径无效");
        return;
    }

    QTemporaryFile imageFile(QDir::tempPath() + "/screenshot-ocr-XXXXXX.png");
    imageFile.setAutoRemove(false);
    if (!imageFile.open() || !pixmap.save(&imageFile, "PNG")) {
        callback(false, QJsonArray(), "无法写入 OCR 临时图片");
        return;
    }

    imageFile.flush();
    currentImagePath = imageFile.fileName();
    imageFile.close();
    stdoutBuffer.clear();
    stderrBuffer.clear();
    currentCallback = callback;
    requestInFlight = true;

    process->setProgram(executable.absoluteFilePath());
    process->setArguments({QString("-image_path=%1").arg(currentImagePath)});
    process->setWorkingDirectory(executable.absolutePath());
    process->start();

    if (!process->waitForStarted(1000)) {
        finishRequest(false, QJsonArray(), process->errorString());
        return;
    }

    timeoutTimer->start(qMax(1000, cfg.localOcrTimeoutMs));
}

void LocalOcrManager::finishRequest(bool success, const QJsonArray &ocrResults, const QString &error) {
    if (!requestInFlight) return;
    requestInFlight = false;
    timeoutTimer->stop();

    if (!currentImagePath.isEmpty()) {
        QFile::remove(currentImagePath);
        currentImagePath.clear();
    }

    auto callback = currentCallback;
    currentCallback = nullptr;
    if (callback) {
        callback(success, ocrResults, error);
    }
}

QJsonArray LocalOcrManager::normalizeResult(const QByteArray &data, bool *ok) const {
    *ok = false;
    
    QList<QByteArray> lines = data.split('\n');
    QJsonDocument doc;
    QJsonParseError parseError;
    
    for (int i = lines.size() - 1; i >= 0; --i) {
        QByteArray line = lines[i].trimmed();
        if (line.startsWith('{') && line.endsWith('}')) {
            doc = QJsonDocument::fromJson(line, &parseError);
            if (parseError.error == QJsonParseError::NoError && doc.isObject()) {
                break;
            }
        }
    }

    if (doc.isNull() || !doc.isObject()) {
        return QJsonArray();
    }

    QJsonObject root = doc.object();
    if (root.value("status").toString() == "success" && root.value("ocr").isArray()) {
        *ok = true;
        return root.value("ocr").toArray();
    }

    if (root.value("code").toInt() == 101) {
        *ok = true;
        return QJsonArray();
    }

    QJsonArray source;
    if (root.value("data").isArray()) {
        source = root.value("data").toArray();
    } else if (root.value("result").isArray()) {
        source = root.value("result").toArray();
    } else {
        return QJsonArray();
    }

    QJsonArray normalized;
    for (const QJsonValue &value : source) {
        QJsonObject item = value.toObject();
        QJsonArray box = item.value("box").toArray();
        if (box.isEmpty()) {
            box = item.value("position").toArray();
        }
        QString text = item.value("text").toString(item.value("str").toString());
        if (text.isEmpty() || box.size() < 4) continue;

        QJsonObject normalizedItem;
        normalizedItem["text"] = text;
        normalizedItem["box"] = box;
        normalized.append(normalizedItem);
    }

    *ok = true;
    return normalized;
}
