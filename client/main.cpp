#include <QApplication>
#include <QSystemTrayIcon>
#include <QMenu>
#include <QMessageBox>
#include <QStyle>
#include <QWidget>
#include <functional>
#include "screenshotwindow.h"
#include "settingspanel.h"

#ifdef Q_OS_WIN
#include <windows.h>
#include <QStringList>

bool parseHotkeyString(const QString &hotkeyStr, UINT &fsModifiers, UINT &vk) {
    fsModifiers = 0;
    vk = 0;
    
    QString cleanStr = hotkeyStr.trimmed();
    if (cleanStr.isEmpty()) return false;
    
    QStringList parts = cleanStr.split("+", Qt::SkipEmptyParts);
    for (int i = 0; i < parts.size(); ++i) {
        QString part = parts[i].trimmed().toUpper();
        if (part == "CTRL" || part == "CONTROL") {
            fsModifiers |= MOD_CONTROL;
        } else if (part == "ALT") {
            fsModifiers |= MOD_ALT;
        } else if (part == "SHIFT") {
            fsModifiers |= MOD_SHIFT;
        } else if (part == "WIN" || part == "META") {
            fsModifiers |= MOD_WIN;
        } else {
            if (part.length() == 1) {
                QChar ch = part.at(0);
                if (ch.isLetterOrNumber()) {
                    vk = ch.toUpper().unicode();
                }
            } else if (part.startsWith("F") && part.length() > 1) {
                bool ok = false;
                int fNum = part.mid(1).toInt(&ok);
                if (ok && fNum >= 1 && fNum <= 24) {
                    vk = VK_F1 + (fNum - 1);
                }
            } else if (part == "SPACE") {
                vk = VK_SPACE;
            } else if (part == "TAB") {
                vk = VK_TAB;
            } else if (part == "ENTER" || part == "RETURN") {
                vk = VK_RETURN;
            } else if (part == "ESCAPE" || part == "ESC") {
                vk = VK_ESCAPE;
            } else if (part == "PRINTSCREEN" || part == "SNAPSHOT" || part == "PRINT") {
                vk = VK_SNAPSHOT;
            } else if (part == "DELETE" || part == "DEL") {
                vk = VK_DELETE;
            } else if (part == "INSERT" || part == "INS") {
                vk = VK_INSERT;
            } else if (part == "HOME") {
                vk = VK_HOME;
            } else if (part == "END") {
                vk = VK_END;
            } else if (part == "PAGEUP" || part == "PGUP") {
                vk = VK_PRIOR;
            } else if (part == "PAGEDOWN" || part == "PGDN") {
                vk = VK_NEXT;
            } else if (part == "UP") {
                vk = VK_UP;
            } else if (part == "DOWN") {
                vk = VK_DOWN;
            } else if (part == "LEFT") {
                vk = VK_LEFT;
            } else if (part == "RIGHT") {
                vk = VK_RIGHT;
            }
        }
    }
    return (vk != 0);
}
#endif

class HotkeyHelper : public QWidget {
public:
    HotkeyHelper(std::function<void()> onTrigger) : triggerCallback(onTrigger) {
        registerCurrentHotkey();
    }
    ~HotkeyHelper() {
        unregisterCurrentHotkey();
    }
    
    void updateHotkey() {
        unregisterCurrentHotkey();
        registerCurrentHotkey();
    }
protected:
    bool nativeEvent(const QByteArray &eventType, void *message, qintptr *result) override {
#ifdef Q_OS_WIN
        MSG *msg = static_cast<MSG*>(message);
        if (msg->message == WM_HOTKEY && msg->wParam == 1) {
            triggerCallback();
            return true;
        }
#endif
        return QWidget::nativeEvent(eventType, message, result);
    }
private:
    void registerCurrentHotkey() {
#ifdef Q_OS_WIN
        ClientConfig config;
        config.load();
        UINT fsModifiers = 0;
        UINT vk = 0;
        if (parseHotkeyString(config.hotkey, fsModifiers, vk)) {
            RegisterHotKey((HWND)this->winId(), 1, fsModifiers, vk);
        } else {
            RegisterHotKey((HWND)this->winId(), 1, MOD_ALT, 0x41);
        }
#endif
    }
    
    void unregisterCurrentHotkey() {
#ifdef Q_OS_WIN
        UnregisterHotKey((HWND)this->winId(), 1);
#endif
    }
    
    std::function<void()> triggerCallback;
};

HotkeyHelper *g_hotkeyHelper = nullptr;
QSystemTrayIcon *g_trayIcon = nullptr;

void updateGlobalHotkey() {
    if (g_hotkeyHelper) {
        g_hotkeyHelper->updateHotkey();
    }
    if (g_trayIcon) {
        ClientConfig config;
        config.load();
        g_trayIcon->setToolTip(QString("YSN 截图翻译 (双击截图 / %1)").arg(config.hotkey));
    }
}

int main(int argc, char *argv[]) {
    QApplication a(argc, argv);
    a.setQuitOnLastWindowClosed(false); // 保证关闭截图窗口时不退出后台程序
    
    // 初始化并加载客户端配置
    ClientConfig config;
    config.load();
    
    // 创建系统托盘并加载应用图标资源
    QIcon appIcon(":/app.ico");
    a.setWindowIcon(appIcon);
    
    g_trayIcon = new QSystemTrayIcon(&a);
    g_trayIcon->setIcon(appIcon);
    g_trayIcon->setToolTip(QString("YSN 截图翻译 (双击截图 / %1)").arg(config.hotkey));
    
    // 托盘菜单
    QMenu *trayMenu = new QMenu();
    QAction *settingsAct = trayMenu->addAction("设置面板");
    trayMenu->addSeparator();
    QAction *quitAct = trayMenu->addAction("退出");
    
    g_trayIcon->setContextMenu(trayMenu);
    g_trayIcon->show();
    
    // 创建全局快捷键辅助窗口
    g_hotkeyHelper = new HotkeyHelper([]() {
        if (SettingsPanel::activeInstance) {
            SettingsPanel::activeInstance->close();
        }
        new ScreenshotWindow();
    });
    
    // 连接信号
    QObject::connect(settingsAct, &QAction::triggered, []() {
        SettingsPanel panel;
        panel.exec();
    });
    
    QObject::connect(quitAct, &QAction::triggered, &a, &QApplication::quit);
    
    // 双击托盘图标直接触发截图
    QObject::connect(g_trayIcon, &QSystemTrayIcon::activated, [](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::DoubleClick || reason == QSystemTrayIcon::Trigger) {
            if (SettingsPanel::activeInstance) {
                SettingsPanel::activeInstance->close();
            }
            new ScreenshotWindow();
        }
    });
    
    // 启动时在托盘气泡中弹出欢迎语
    g_trayIcon->showMessage(
        "YSN 截图翻译已在后台运行",
        QString("使用快捷键 %1，或双击托盘图标即可开始框选翻译！").arg(config.hotkey),
        QSystemTrayIcon::Information,
        5000
    );
    
    int ret = a.exec();
    delete g_hotkeyHelper;
    return ret;
}
