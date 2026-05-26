#pragma once
#include <QWidget>
#include <QPixmap>
#include <QPoint>
#include <QRect>
#include <QPushButton>
#include <QHBoxLayout>
#include <QLabel>
#include "config.h"
#include "networkclient.h"

// 悬浮工具栏组件
class FloatingToolbar : public QWidget {
    Q_OBJECT
public:
    explicit FloatingToolbar(QWidget *parent = nullptr);
signals:
    void translateRequested();
    void copyRequested();
    void saveRequested();
    void pinRequested();
    void settingsRequested();
    void closeRequested();
};

class ScreenshotWindow : public QWidget {
    Q_OBJECT
public:
    explicit ScreenshotWindow(QWidget *parent = nullptr);
    ~ScreenshotWindow() override;

protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

private:
    void showToolbar();
    void hideToolbar();
    void updateToolbarPosition();
    void triggerTranslation();
    void copyToClipboard();
    void saveToFile();
    void pinImage();

    QPixmap fullScreenPixmap;
    QPixmap currentImage; // 可能是原图，也可能是翻译后的图片
    bool isDragging = false;
    bool isTranslated = false;
    bool isLoading = false;
    
    QPoint startPoint;
    QPoint endPoint;
    QRect croppedRect;
    
    FloatingToolbar *toolbar = nullptr;
    NetworkClient *netClient = nullptr;
    ClientConfig config;
};

// 钉图窗口（Pin Window）组件
class PinWindow : public QWidget {
    Q_OBJECT
public:
    explicit PinWindow(const QPixmap &pixmap, const QPoint &pos, QWidget *parent = nullptr);
protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void contextMenuEvent(QContextMenuEvent *event) override;
private:
    QPixmap m_pixmap;
    QPoint m_dragPosition;
};
