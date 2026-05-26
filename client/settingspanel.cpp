#include "settingspanel.h"
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMessageBox>
#include <QVBoxLayout>
#include <QSettings>
#include <QCoreApplication>
#include <QDir>
#include <QStackedWidget>

SettingsPanel* SettingsPanel::activeInstance = nullptr;

SettingsPanel::SettingsPanel(QWidget *parent) : QDialog(parent) {
    activeInstance = this;
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowFlags(Qt::Window | Qt::WindowMinMaxButtonsHint | Qt::WindowCloseButtonHint);
    setWindowTitle("配置中心");
    resize(660, 480);
    
    // Set premium modern styles matching '系统盘清理'
    setStyleSheet(
        "QDialog {"
        "  background-color: #f0f4f8;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "}"
        "QLabel {"
        "  color: #64748b;"
        "  font-size: 12px;"
        "  font-weight: 500;"
        "}"
        "QLineEdit {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 6px;"
        "  padding: 6px 10px;"
        "  color: #1e293b;"
        "  font-size: 12px;"
        "}"
        "QLineEdit:focus {"
        "  border: 1px solid #3b82f6;"
        "}"
        "QComboBox {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 6px;"
        "  padding: 5px 10px;"
        "  color: #1e293b;"
        "  font-size: 12px;"
        "}"
        "QSpinBox {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 6px;"
        "  padding: 5px 10px;"
        "  color: #1e293b;"
        "  font-size: 12px;"
        "}"
        "QPushButton {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 6px;"
        "  padding: 6px 12px;"
        "  color: #1e293b;"
        "  font-size: 12px;"
        "  font-weight: 500;"
        "}"
        "QPushButton:hover {"
        "  border-color: #3b82f6;"
        "  color: #3b82f6;"
        "  background-color: #eff6ff;"
        "}"
        "QPushButton#saveBtn {"
        "  background: qlineargradient(x1:0, y1:0, x2:1, y2:1, stop:0 #2563eb, stop:1 #3b82f6);"
        "  border: none;"
        "  color: #ffffff;"
        "  font-weight: 600;"
        "}"
        "QPushButton#saveBtn:hover {"
        "  background: qlineargradient(x1:0, y1:0, x2:1, y2:1, stop:0 #1d4ed8, stop:1 #2563eb);"
        "}"
        "QListWidget {"
        "  background-color: #0f172a;"
        "  border: none;"
        "  border-right: 1px solid rgba(255, 255, 255, 0.08);"
        "}"
        "QListWidget::item {"
        "  height: 38px;"
        "  padding-left: 12px;"
        "  margin: 4px 8px;"
        "  border-radius: 6px;"
        "  color: #94a3b8;"
        "  font-weight: 600;"
        "}"
        "QListWidget::item:hover {"
        "  background-color: rgba(255, 255, 255, 0.05);"
        "  color: #f1f5f9;"
        "}"
        "QListWidget::item:selected {"
        "  background-color: rgba(59, 130, 246, 0.15);"
        "  color: #3b82f6;"
        "  font-weight: 700;"
        "}"
        "QGroupBox {"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 8px;"
        "  margin-top: 12px;"
        "  background-color: #ffffff;"
        "  font-weight: bold;"
        "  color: #1e293b;"
        "  padding: 15px;"
        "}"
        "QGroupBox:disabled {"
        "  background-color: #f8fafc;"
        "  border-color: #e2e8f0;"
        "  color: #94a3b8;"
        "}"
        "QGroupBox::title {"
        "  subcontrol-origin: margin;"
        "  left: 10px;"
        "  padding: 0 4px;"
        "  color: #1e293b;"
        "}"
        "QLineEdit:disabled, QComboBox:disabled, QSpinBox:disabled {"
        "  background-color: #f8fafc;"
        "  color: #cbd5e0;"
        "  border-color: #e2e8f0;"
        "}"
        "QLabel:disabled {"
        "  color: #cbd5e0;"
        "}"
        "QStackedWidget {"
        "  background-color: #f0f4f8;"
        "}"
        "QWidget#page1, QWidget#page2, QWidget#page3, QWidget#page4, QWidget#page5 {"
        "  background-color: #f0f4f8;"
        "}"
    );
    
    config.load();
    netClient = new NetworkClient(this);
    
    // Left Sidebar Navigation
    sidebarList = new QListWidget(this);
    sidebarList->setFixedWidth(150);
    sidebarList->addItem("🌐 服务与通道");
    sidebarList->addItem("🔍 本地 OCR");
    sidebarList->addItem("⌨ 全局热键");
    sidebarList->addItem("⚙ 系统设置");
    sidebarList->addItem("ℹ 关于软件");
    
    // Right Pages Container
    stackedWidget = new QStackedWidget(this);
    
    // ----------------------------------------
    // Page 1: 服务与通道
    // ----------------------------------------
    QWidget *page1 = new QWidget(this);
    page1->setObjectName("page1");
    QVBoxLayout *layout1 = new QVBoxLayout(page1);
    layout1->setContentsMargins(20, 20, 20, 20);
    layout1->setSpacing(12);
    
    QGroupBox *serverGroup = new QGroupBox("N100 服务器设置", page1);
    QFormLayout *serverForm = new QFormLayout(serverGroup);
    serverForm->setSpacing(8);
    
    QLabel *hint1 = new QLabel("内网直连: http://192.168.1.3:8318", page1);
    hint1->setStyleSheet("color: #718096; font-size: 11px;");
    serverForm->addRow(hint1);
    
    serverUrlEdit = new QLineEdit(config.serverUrl, page1);
    serverForm->addRow("服务器地址:", serverUrlEdit);
    
    clientTokenEdit = new QLineEdit(config.clientToken, page1);
    clientTokenEdit->setEchoMode(QLineEdit::Password);
    serverForm->addRow("鉴权 Token:", clientTokenEdit);
    layout1->addWidget(serverGroup);
    
    QGroupBox *transGroup = new QGroupBox("翻译通道设置", page1);
    QVBoxLayout *transVBox = new QVBoxLayout(transGroup);
    
    channelCombo = new QComboBox(page1);
    channelCombo->addItems({"new-api", "baidu", "google"});
    channelCombo->setCurrentText(config.channel);
    transVBox->addWidget(channelCombo);
    
    // new-api Panel
    QGroupBox *llmSubGroup = new QGroupBox("new-api (LLM 模式)", page1);
    QFormLayout *llmForm = new QFormLayout(llmSubGroup);
    
    QLabel *hint2 = new QLabel("内网中转: http://192.168.1.3:3001", page1);
    hint2->setStyleSheet("color: #718096; font-size: 11px;");
    llmForm->addRow(hint2);
    
    newApiBaseEdit = new QLineEdit(config.newApiBase, page1);
    llmForm->addRow("中转地址:", newApiBaseEdit);
    
    newApiKeyEdit = new QLineEdit(config.newApiKey, page1);
    newApiKeyEdit->setEchoMode(QLineEdit::Password);
    llmForm->addRow("API Key:", newApiKeyEdit);
    
    QHBoxLayout *modelHBox = new QHBoxLayout();
    newApiModelCombo = new QComboBox(page1);
    newApiModelCombo->addItem(config.newApiModel);
    newApiModelCombo->setEditable(true);
    modelHBox->addWidget(newApiModelCombo);
    
    fetchModelsBtn = new QPushButton("获取模型", page1);
    modelHBox->addWidget(fetchModelsBtn);
    llmForm->addRow("模型名称:", modelHBox);
    transVBox->addWidget(llmSubGroup);
    
    // Baidu Panel
    QGroupBox *baiduSubGroup = new QGroupBox("百度翻译", page1);
    QFormLayout *baiduForm = new QFormLayout(baiduSubGroup);
    baiduAppIdEdit = new QLineEdit(config.baiduAppId, page1);
    baiduSecretKeyEdit = new QLineEdit(config.baiduSecretKey, page1);
    baiduForm->addRow("AppID:", baiduAppIdEdit);
    baiduForm->addRow("密钥:", baiduSecretKeyEdit);
    transVBox->addWidget(baiduSubGroup);
    
    layout1->addWidget(transGroup);
    layout1->addStretch();
    stackedWidget->addWidget(page1);
    
    // ----------------------------------------
    // Page 2: 本地 OCR
    // ----------------------------------------
    QWidget *page2 = new QWidget(this);
    page2->setObjectName("page2");
    QVBoxLayout *layout2 = new QVBoxLayout(page2);
    layout2->setContentsMargins(20, 20, 20, 20);
    layout2->setSpacing(12);
    
    QGroupBox *localOcrGroup = new QGroupBox("本地 OCR 引擎设置", page2);
    QFormLayout *localOcrForm = new QFormLayout(localOcrGroup);
    localOcrForm->setSpacing(12);
    
    useLocalOcrCheck = new SwitchButton(page2);
    useLocalOcrCheck->setChecked(config.useLocalOcr);
    localOcrForm->addRow("优先使用本地 OCR:", useLocalOcrCheck);
    
    QHBoxLayout *localOcrPathHBox = new QHBoxLayout();
    localOcrPathEdit = new QLineEdit(config.localOcrExecutablePath, page2);
    browseLocalOcrBtn = new QPushButton("浏览...", page2);
    localOcrPathHBox->addWidget(localOcrPathEdit);
    localOcrPathHBox->addWidget(browseLocalOcrBtn);
    localOcrForm->addRow("引擎路径:", localOcrPathHBox);
    
    localOcrTimeoutSpin = new QSpinBox(page2);
    localOcrTimeoutSpin->setRange(1000, 60000);
    localOcrTimeoutSpin->setSingleStep(500);
    localOcrTimeoutSpin->setValue(config.localOcrTimeoutMs);
    localOcrTimeoutSpin->setSuffix(" ms");
    localOcrForm->addRow("超时时间:", localOcrTimeoutSpin);
    
    fallbackToRemoteOcrCheck = new SwitchButton(page2);
    fallbackToRemoteOcrCheck->setChecked(config.fallbackToRemoteOcr);
    localOcrForm->addRow("本地失败回退云端:", fallbackToRemoteOcrCheck);
    
    layout2->addWidget(localOcrGroup);
    layout2->addStretch();
    stackedWidget->addWidget(page2);
    
    // ----------------------------------------
    // Page 3: 全局热键
    // ----------------------------------------
    QWidget *page3 = new QWidget(this);
    page3->setObjectName("page3");
    QVBoxLayout *layout3 = new QVBoxLayout(page3);
    layout3->setContentsMargins(20, 20, 20, 20);
    layout3->setSpacing(12);
    
    QGroupBox *hotkeyGroup = new QGroupBox("全局快捷键", page3);
    QFormLayout *hotkeyForm = new QFormLayout(hotkeyGroup);
    hotkeyForm->setSpacing(12);
    
    hotkeyEdit = new QKeySequenceEdit(page3);
    hotkeyForm->addRow("截图翻译快捷键:", hotkeyEdit);
    
    layout3->addWidget(hotkeyGroup);
    layout3->addStretch();
    stackedWidget->addWidget(page3);
    
    // ----------------------------------------
    // Page 4: 系统设置
    // ----------------------------------------
    QWidget *page4 = new QWidget(this);
    page4->setObjectName("page4");
    QVBoxLayout *layout4 = new QVBoxLayout(page4);
    layout4->setContentsMargins(20, 20, 20, 20);
    layout4->setSpacing(12);
    
    QGroupBox *systemGroup = new QGroupBox("系统设置项", page4);
    QFormLayout *systemForm = new QFormLayout(systemGroup);
    systemForm->setSpacing(12);
    
    autostartCheck = new SwitchButton(page4);
    autostartCheck->setChecked(isAutostartEnabled());
    systemForm->addRow("开机自动启动:", autostartCheck);
    
    layout4->addWidget(systemGroup);
    layout4->addStretch();
    stackedWidget->addWidget(page4);
    
    // ----------------------------------------
    // Page 5: 关于软件
    // ----------------------------------------
    QWidget *page5 = new QWidget(this);
    page5->setObjectName("page5");
    QVBoxLayout *layout5 = new QVBoxLayout(page5);
    layout5->setContentsMargins(20, 20, 20, 20);
    layout5->setSpacing(16);
    
    QFrame *aboutCard = new QFrame(page5);
    aboutCard->setStyleSheet(
        "QFrame {"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 8px;"
        "  background-color: #ffffff;"
        "  padding: 20px;"
        "}"
    );
    QVBoxLayout *cardLayout = new QVBoxLayout(aboutCard);
    cardLayout->setSpacing(10);
    
    QLabel *appNameLabel = new QLabel("YSN 截图翻译", aboutCard);
    appNameLabel->setStyleSheet("font-size: 20px; font-weight: bold; color: #1e293b;");
    
    QLabel *versionLabel = new QLabel("版本: v0.4.0 (Windows x64)", aboutCard);
    versionLabel->setStyleSheet("font-size: 12px; color: #64748b;");
    
    QLabel *descLabel = new QLabel(
        "本软件是一款基于 Qt C++ 设计的极轻量截图翻译系统。\n"
        "AI 视觉推理、自动背景环形采样中位数去杂色、Pillow 字号动态换行嵌字全部内聚于 N100 服务器处理。\n"
        "客户端仅需保持秒级唤醒，支持高精度 DWM Snap 窗口边界动画感知及多功能图形标注操作！",
        aboutCard
    );
    descLabel->setStyleSheet("font-size: 12px; line-height: 1.6; color: #64748b;");
    
    cardLayout->addWidget(appNameLabel);
    cardLayout->addWidget(versionLabel);
    cardLayout->addWidget(descLabel);
    
    layout5->addWidget(aboutCard);
    layout5->addStretch();
    stackedWidget->addWidget(page5);
    
    // ----------------------------------------
    // Layout assembly & Global control panel
    // ----------------------------------------
    QVBoxLayout *dialogMainLayout = new QVBoxLayout();
    dialogMainLayout->setContentsMargins(0, 0, 0, 0);
    dialogMainLayout->setSpacing(0);
    
    QHBoxLayout *bodyLayout = new QHBoxLayout();
    bodyLayout->setContentsMargins(0, 0, 0, 0);
    bodyLayout->setSpacing(0);
    bodyLayout->addWidget(sidebarList);
    bodyLayout->addWidget(stackedWidget);
    dialogMainLayout->addLayout(bodyLayout);
    
    // Bottom Buttons Container
    QFrame *bottomFrame = new QFrame(this);
    bottomFrame->setStyleSheet(
        "QFrame {"
        "  border-top: 1px solid #e2e8f0;"
        "  background-color: #ffffff;"
        "}"
    );
    bottomFrame->setFixedHeight(50);
    QHBoxLayout *bottomLayout = new QHBoxLayout(bottomFrame);
    bottomLayout->setContentsMargins(20, 0, 20, 0);
    
    verifyBtn = new QPushButton("点击验证", bottomFrame);
    bottomLayout->addWidget(verifyBtn);
    
    statusLabel = new QLabel("", bottomFrame);
    statusLabel->setStyleSheet("font-weight: bold; font-size: 11px;");
    bottomLayout->addWidget(statusLabel);
    
    bottomLayout->addStretch();
    
    QPushButton *cancelBtn = new QPushButton("取消", bottomFrame);
    QPushButton *saveBtn = new QPushButton("保存并应用", bottomFrame);
    saveBtn->setObjectName("saveBtn");
    
    bottomLayout->addWidget(cancelBtn);
    bottomLayout->addWidget(saveBtn);
    
    dialogMainLayout->addWidget(bottomFrame);
    
    // Make dialog use the compiled layout
    setLayout(dialogMainLayout);
    
    // Connect sidebar list selection to page swapper
    connect(sidebarList, &QListWidget::currentRowChanged, stackedWidget, &QStackedWidget::setCurrentIndex);
    sidebarList->setCurrentRow(0);
    
    // Channel visibility logic
    auto updateVisibility = [=]() {
        llmSubGroup->setEnabled(channelCombo->currentText() == "new-api");
        baiduSubGroup->setEnabled(channelCombo->currentText() == "baidu");
        llmSubGroup->setVisible(true);
        baiduSubGroup->setVisible(true);
    };
    connect(channelCombo, &QComboBox::currentTextChanged, updateVisibility);
    updateVisibility();
    
    // Connections
    connect(browseLocalOcrBtn, &QPushButton::clicked, [=]() {
        QString path = QFileDialog::getOpenFileName(
            this,
            "选择 PaddleOCR-json 引擎",
            localOcrPathEdit->text(),
            "Executable (*.exe);;All Files (*.*)"
        );
        if (!path.isEmpty()) {
            localOcrPathEdit->setText(path);
        }
    });
    
    connect(fetchModelsBtn, &QPushButton::clicked, [=]() {
        fetchModelsBtn->setEnabled(false);
        statusLabel->setText("正在拉取模型...");
        statusLabel->setStyleSheet("color: #3182ce;");
        
        saveFields();
        netClient->fetchModels(config, newApiBaseEdit->text(), newApiKeyEdit->text(), [=](bool ok, const QStringList &models) {
            fetchModelsBtn->setEnabled(true);
            if (ok && !models.isEmpty()) {
                newApiModelCombo->clear();
                newApiModelCombo->addItems(models);
                statusLabel->setText("获取模型成功！已更新下拉框。");
                statusLabel->setStyleSheet("color: #38a169;");
            } else {
                statusLabel->setText("获取模型失败，请检查网络。");
                statusLabel->setStyleSheet("color: #e53e3e;");
            }
        });
    });
    
    connect(verifyBtn, &QPushButton::clicked, [=]() {
        verifyBtn->setEnabled(false);
        statusLabel->setText("正在验证配置...");
        statusLabel->setStyleSheet("color: #3182ce;");
        
        saveFields();
        QJsonObject payload;
        payload["channel"] = config.channel;
        
        QJsonObject channelsConfig;
        if (config.channel == "new-api") {
            channelsConfig["base_url"] = config.newApiBase;
            channelsConfig["api_key"] = config.newApiKey;
            channelsConfig["model"] = newApiModelCombo->currentText();
        } else if (config.channel == "baidu") {
            channelsConfig["app_id"] = config.baiduAppId;
            channelsConfig["secret_key"] = config.baiduSecretKey;
        }
        payload["config"] = channelsConfig;
        
        netClient->testConfig(config, payload, [=](bool ok, const QString &msg) {
            verifyBtn->setEnabled(true);
            if (ok) {
                statusLabel->setText("验证成功: " + msg);
                statusLabel->setStyleSheet("color: #38a169;");
            } else {
                statusLabel->setText("验证失败: " + msg);
                statusLabel->setStyleSheet("color: #e53e3e;");
            }
        });
    });
    
    connect(saveBtn, &QPushButton::clicked, [=]() {
        saveFields();
        if (config.useLocalOcr) {
            QFileInfo engineFile(config.localOcrExecutablePath);
            if (!engineFile.exists() || !engineFile.isFile() || !engineFile.isExecutable() || engineFile.isRelative()) {
                QMessageBox::warning(this, "本地 OCR 配置错误", "启用本地 OCR 时必须选择有效的 PaddleOCR-json.exe 绝对路径。");
                return;
            }
        }
        config.save();
        setAutostartEnabled(autostartCheck->isChecked());
        updateGlobalHotkey();
        QMessageBox::information(this, "保存成功", "配置已成功保存并应用！");
        accept();
    });
    
    connect(cancelBtn, &QPushButton::clicked, this, &QDialog::reject);
    loadFields();
}

void SettingsPanel::loadFields() {
    config.load();
    serverUrlEdit->setText(config.serverUrl);
    clientTokenEdit->setText(config.clientToken);
    channelCombo->setCurrentText(config.channel);
    newApiBaseEdit->setText(config.newApiBase);
    newApiKeyEdit->setText(config.newApiKey);
    newApiModelCombo->clear();
    newApiModelCombo->addItem(config.newApiModel);
    baiduAppIdEdit->setText(config.baiduAppId);
    baiduSecretKeyEdit->setText(config.baiduSecretKey);
    useLocalOcrCheck->setChecked(config.useLocalOcr);
    localOcrPathEdit->setText(config.localOcrExecutablePath);
    localOcrTimeoutSpin->setValue(config.localOcrTimeoutMs);
    fallbackToRemoteOcrCheck->setChecked(config.fallbackToRemoteOcr);
    hotkeyEdit->setKeySequence(QKeySequence(config.hotkey));
    autostartCheck->setChecked(isAutostartEnabled());
}

void SettingsPanel::saveFields() {
    config.serverUrl = serverUrlEdit->text();
    config.clientToken = clientTokenEdit->text();
    config.channel = channelCombo->currentText();
    config.newApiBase = newApiBaseEdit->text();
    config.newApiKey = newApiKeyEdit->text();
    config.newApiModel = newApiModelCombo->currentText();
    config.baiduAppId = baiduAppIdEdit->text();
    config.baiduSecretKey = baiduSecretKeyEdit->text();
    config.useLocalOcr = useLocalOcrCheck->isChecked();
    config.localOcrExecutablePath = localOcrPathEdit->text();
    config.localOcrTimeoutMs = localOcrTimeoutSpin->value();
    config.fallbackToRemoteOcr = fallbackToRemoteOcrCheck->isChecked();
    
    QString hotkeyStr = hotkeyEdit->keySequence().toString().split(",").first().trimmed();
    if (!hotkeyStr.isEmpty()) {
        config.hotkey = hotkeyStr;
    }
}

bool SettingsPanel::isAutostartEnabled() {
#ifdef Q_OS_WIN
    QSettings settings("HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", QSettings::NativeFormat);
    return settings.contains("ScreenshotTranslator");
#else
    return false;
#endif
}

void SettingsPanel::setAutostartEnabled(bool enabled) {
#ifdef Q_OS_WIN
    QSettings settings("HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", QSettings::NativeFormat);
    if (enabled) {
        QString appPath = QDir::toNativeSeparators(QCoreApplication::applicationFilePath());
        settings.setValue("ScreenshotTranslator", "\"" + appPath + "\"");
    } else {
        settings.remove("ScreenshotTranslator");
    }
#endif
}

SettingsPanel::~SettingsPanel() {
    if (activeInstance == this) {
        activeInstance = nullptr;
    }
}
