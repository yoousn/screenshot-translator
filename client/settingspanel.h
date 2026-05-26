#pragma once
#include <QDialog>
#include <QLineEdit>
#include <QComboBox>
#include <QPushButton>
#include <QLabel>
#include "config.h"
#include "networkclient.h"

class SettingsPanel : public QDialog {
    Q_OBJECT
public:
    explicit SettingsPanel(QWidget *parent = nullptr);
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

    QPushButton *verifyBtn;
    QLabel *statusLabel;
    
    ClientConfig config;
    NetworkClient *netClient;
    
    void loadFields();
    void saveFields();
};
