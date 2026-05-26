#pragma once
#include <QDialog>
#include <QLineEdit>
#include <QComboBox>
#include <QPushButton>
#include <QLabel>
#include <QSpinBox>
#include <QKeySequenceEdit>
#include <QListWidget>
#include <QStackedWidget>
#include "config.h"
#include "networkclient.h"
#include "switchbutton.h"

class SettingsPanel : public QDialog {
    Q_OBJECT
public:
    explicit SettingsPanel(QWidget *parent = nullptr);
    ~SettingsPanel() override;
    static SettingsPanel* activeInstance;
private:
    QListWidget *sidebarList;
    QStackedWidget *stackedWidget;

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
    SwitchButton *useLocalOcrCheck;
    QLineEdit *localOcrPathEdit;
    QPushButton *browseLocalOcrBtn;
    QSpinBox *localOcrTimeoutSpin;
    SwitchButton *fallbackToRemoteOcrCheck;

    // Hotkey settings
    QKeySequenceEdit *hotkeyEdit;

    // System settings
    SwitchButton *autostartCheck;

    QPushButton *verifyBtn;
    QLabel *statusLabel;
    
    ClientConfig config;
    NetworkClient *netClient;
    
    void loadFields();
    void saveFields();
    bool isAutostartEnabled();
    void setAutostartEnabled(bool enabled);
};
