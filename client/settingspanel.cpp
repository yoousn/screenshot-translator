#include "settingspanel.h"
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QFormLayout>
#include <QGroupBox>
#include <QJsonObject>
#include <QJsonDocument>

SettingsPanel* SettingsPanel::activeInstance = nullptr;

SettingsPanel::SettingsPanel(QWidget *parent) : QDialog(parent) {
    activeInstance = this;
    setWindowTitle("截图翻译配置面板");
    resize(550, 480);
    
    config.load();
    netClient = new NetworkClient(this);
    
    QVBoxLayout *mainLayout = new QVBoxLayout(this);
    
    // 1. 服务器设置
    QGroupBox *serverGroup = new QGroupBox("N100 服务器设置", this);
    QFormLayout *serverForm = new QFormLayout(serverGroup);
    
    QLabel *hint1 = new QLabel("内网: http://192.168.1.3:8318", this);
    hint1->setStyleSheet("color: gray; font-family: 'Segoe UI'; font-size: 11px;");
    serverForm->addRow(hint1);
    
    serverUrlEdit = new QLineEdit(config.serverUrl, this);
    serverForm->addRow("服务器地址:", serverUrlEdit);
    
    clientTokenEdit = new QLineEdit(config.clientToken, this);
    clientTokenEdit->setEchoMode(QLineEdit::Password);
    serverForm->addRow("鉴权 Token:", clientTokenEdit);
    
    mainLayout->addWidget(serverGroup);
    
    // 2. 翻译设置
    QGroupBox *transGroup = new QGroupBox("翻译通道设置", this);
    QVBoxLayout *transVBox = new QVBoxLayout(transGroup);
    
    channelCombo = new QComboBox(this);
    channelCombo->addItems({"new-api", "baidu", "google"});
    channelCombo->setCurrentText(config.channel);
    transVBox->addWidget(channelCombo);
    
    // new-api Group
    QGroupBox *llmSubGroup = new QGroupBox("new-api (LLM 模式)", this);
    QFormLayout *llmForm = new QFormLayout(llmSubGroup);
    
    QLabel *hint2 = new QLabel("内网: http://192.168.1.3:3001", this);
    hint2->setStyleSheet("color: gray; font-family: 'Segoe UI'; font-size: 11px;");
    llmForm->addRow(hint2);
    
    newApiBaseEdit = new QLineEdit(config.newApiBase, this);
    llmForm->addRow("中转地址:", newApiBaseEdit);
    
    newApiKeyEdit = new QLineEdit(config.newApiKey, this);
    newApiKeyEdit->setEchoMode(QLineEdit::Password);
    llmForm->addRow("API Key:", newApiKeyEdit);
    
    QHBoxLayout *modelHBox = new QHBoxLayout();
    newApiModelCombo = new QComboBox(this);
    newApiModelCombo->addItem(config.newApiModel);
    newApiModelCombo->setEditable(true);
    modelHBox->addWidget(newApiModelCombo);
    
    fetchModelsBtn = new QPushButton("获取模型", this);
    modelHBox->addWidget(fetchModelsBtn);
    llmForm->addRow("模型名称:", modelHBox);
    
    transVBox->addWidget(llmSubGroup);
    
    // 百度翻译组
    QGroupBox *baiduSubGroup = new QGroupBox("百度翻译", this);
    QFormLayout *baiduForm = new QFormLayout(baiduSubGroup);
    baiduAppIdEdit = new QLineEdit(config.baiduAppId, this);
    baiduSecretKeyEdit = new QLineEdit(config.baiduSecretKey, this);
    baiduForm->addRow("AppID:", baiduAppIdEdit);
    baiduForm->addRow("密钥:", baiduSecretKeyEdit);
    
    transVBox->addWidget(baiduSubGroup);
    mainLayout->addWidget(transGroup);
    
    // 3. 测试与保存
    QHBoxLayout *btnHBox = new QHBoxLayout();
    verifyBtn = new QPushButton("点击验证", this);
    btnHBox->addWidget(verifyBtn);
    
    statusLabel = new QLabel("", this);
    statusLabel->setStyleSheet("font-weight: bold; font-size: 12px;");
    btnHBox->addWidget(statusLabel);
    
    btnHBox->addStretch();
    QPushButton *saveBtn = new QPushButton("保存并应用", this);
    QPushButton *cancelBtn = new QPushButton("取消", this);
    btnHBox->addWidget(cancelBtn);
    btnHBox->addWidget(saveBtn);
    mainLayout->addLayout(btnHBox);
    
    // 控制显示联动
    auto updateVisibility = [=]() {
        llmSubGroup->setVisible(channelCombo->currentText() == "new-api");
        baiduSubGroup->setVisible(channelCombo->currentText() == "baidu");
    };
    connect(channelCombo, &QComboBox::currentTextChanged, updateVisibility);
    updateVisibility();
    
    // 连接信号事件
    connect(fetchModelsBtn, &QPushButton::clicked, [=]() {
        fetchModelsBtn->setEnabled(false);
        statusLabel->setText("正在拉取模型...");
        statusLabel->setStyleSheet("color: blue;");
        
        saveFields(); // 更新临时状态
        netClient->fetchModels(config, newApiBaseEdit->text(), newApiKeyEdit->text(), [=](bool ok, const QStringList &models) {
            fetchModelsBtn->setEnabled(true);
            if (ok && !models.isEmpty()) {
                newApiModelCombo->clear();
                newApiModelCombo->addItems(models);
                statusLabel->setText("获取模型成功！已更新下拉框。");
                statusLabel->setStyleSheet("color: green;");
            } else {
                statusLabel->setText("获取模型失败，请检查网络或配置。");
                statusLabel->setStyleSheet("color: red;");
            }
        });
    });
    
    connect(verifyBtn, &QPushButton::clicked, [=]() {
        verifyBtn->setEnabled(false);
        statusLabel->setText("正在验证配置...");
        statusLabel->setStyleSheet("color: blue;");
        
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
                statusLabel->setStyleSheet("color: green;");
            } else {
                statusLabel->setText("验证失败: " + msg);
                statusLabel->setStyleSheet("color: red;");
            }
        });
    });
    
    connect(saveBtn, &QPushButton::clicked, [=]() {
        saveFields();
        config.save();
        accept();
    });
    connect(cancelBtn, &QPushButton::clicked, this, &QDialog::reject);
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
}

SettingsPanel::~SettingsPanel() {
    if (activeInstance == this) {
        activeInstance = nullptr;
    }
}

