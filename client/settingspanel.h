#pragma once
#include <QCheckBox>
#include <QDialog>
#include <QLineEdit>
#include <QComboBox>
#include <QPushButton>
#include <QLabel>
#include <QSpinBox>
#include <QKeySequenceEdit>
#include "config.h"
#include "networkclient.h"

class SettingsPanel : public QDialog {
    Q_OBJECT
public:
    explicit SettingsPanel(QWidget *parent = nullptr);
    ~SettingsPanel() override;
    static SettingsPanel* activeInstance;
private:
    QLineEdit *serverUrlEdit;
    QLineEdit *clientTokenEdit;
    QComboBox *channelCombo;
    
    // new-api settings
    QLineEdit *newApiBaseEdit;
    QLineEdit *newApiKeyEdit;
    QComboBox *newApiModelCombo;
    QPushButton *fetchModelsBtn;

    // Baidu settings
    QLineEdit *baiduAppIdEdit;
    QLineEdit *baiduSecretKeyEdit;

    // Local OCR settings
    QCheckBox *useLocalOcrCheck;
    QLineEdit *localOcrPathEdit;
    QPushButton *browseLocalOcrBtn;
    QSpinBox *localOcrTimeoutSpin;
    QCheckBox *fallbackToRemoteOcrCheck;

    // Hotkey settings
    QKeySequenceEdit *hotkeyEdit;

    QPushButton *verifyBtn;
    QLabel *statusLabel;
    
    ClientConfig config;
    NetworkClient *netClient;
    
    void loadFields();
    void saveFields();
};
