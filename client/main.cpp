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
#endif

class HotkeyHelper : public QWidget {
public:
    HotkeyHelper(std::function<void()> onTrigger) : triggerCallback(onTrigger) {
#ifdef Q_OS_WIN
        // 注册全局快捷键 Alt+A (MOD_ALT = 0x0001, 'A' = 0x41)
        RegisterHotKey((HWND)this->winId(), 1, 0x0001, 0x41);
#endif
    }
    ~HotkeyHelper() {
#ifdef Q_OS_WIN
        UnregisterHotKey((HWND)this->winId(), 1);
#endif
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
    std::function<void()> triggerCallback;
};

int main(int argc, char *argv[]) {
    QApplication a(argc, argv);
    a.setQuitOnLastWindowClosed(false); // 保证关闭截图窗口时不退出后台程序
    
    // 初始化并加载客户端配置
    ClientConfig config;
    config.load();
    
    // 创建系统托盘
    QSystemTrayIcon *trayIcon = new QSystemTrayIcon(&a);
    // 使用 Qt 内置标准图标作为临时图标（高级计算机样式）
    trayIcon->setIcon(a.style()->standardIcon(QStyle::SP_ComputerIcon));
    trayIcon->setToolTip("YSN 截图翻译 (双击截图 / Alt+A)");
    
    // 托盘菜单
    QMenu *trayMenu = new QMenu();
    
    QAction *screenshotAct = trayMenu->addAction("截图翻译");
    screenshotAct->setToolTip("双击托盘图标、点击这里或按 Alt+A 开始截图 (Esc 退出)");
    
    QAction *settingsAct = trayMenu->addAction("设置面板");
    
    trayMenu->addSeparator();
    QAction *quitAct = trayMenu->addAction("退出");
    
    trayIcon->setContextMenu(trayMenu);
    trayIcon->show();
    
    // 创建全局快捷键辅助窗口
    HotkeyHelper *hotkey = new HotkeyHelper([]() {
        new ScreenshotWindow();
    });
    
    // 连接信号
    QObject::connect(screenshotAct, &QAction::triggered, []() {
        // 创建并显示截图窗口
        new ScreenshotWindow();
    });
    
    QObject::connect(settingsAct, &QAction::triggered, []() {
        SettingsPanel panel;
        panel.exec();
    });
    
    QObject::connect(quitAct, &QAction::triggered, &a, &QApplication::quit);
    
    // 双击托盘图标直接触发截图
    QObject::connect(trayIcon, &QSystemTrayIcon::activated, [](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::DoubleClick || reason == QSystemTrayIcon::Trigger) {
            new ScreenshotWindow();
        }
    });
    
    // 启动时在托盘气泡中弹出华丽的欢迎语，提升体验感！
    trayIcon->showMessage(
        "YSN 截图翻译已在后台运行",
        "使用快捷键 Alt+A，或双击托盘图标即可开始框选翻译！",
        QSystemTrayIcon::Information,
        5000
    );
    
    int ret = a.exec();
    delete hotkey;
    return ret;
}
