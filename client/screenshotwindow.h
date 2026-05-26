#pragma once
#include <QWidget>
#include <QPixmap>
#include <QImage>
#include <QPoint>
#include <QRect>
#include <QList>
#include <QColor>
#include <QTimer>
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
    void setTranslateEnabled(bool enabled);
signals:
    void translateRequested();
    void copyRequested();
    void saveRequested();
    void pinRequested();
    void settingsRequested();
    void closeRequested();
protected:
    void keyPressEvent(QKeyEvent *event) override;
private:
    QPushButton *transBtn = nullptr;
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
    void checkForText();

    // 拖动/缩放状态机与手柄
    enum CaptureState { StateIdle, StateSelecting, StateMoving, StateResizing };
    CaptureState captureState = StateIdle;
    QPoint pressPos;
    QPoint dragOffset;
    int activeHandle = -1;
    
    QList<QPoint> getHandlePoints() const;
    int getHandleAt(const QPoint &pos);
    void resizeSelection(const QPoint &pos);
    void updateCursorShape(const QPoint &pos);
    
    // 窗口边界探测与捕捉
    QList<QRect> detectedWindowRects;
    QRect hoveredSnapRect;
    QRect getSnappedRect(const QPoint &pos);
    
    // 屏幕坐标与 RGB 颜色指示器
    QPoint currentMousePos;
    QColor currentColor;
    QImage fullScreenImage;
    void drawHUD(QPainter &painter);
    
    // 旋转加载动画定时器
    int spinnerAngle = 0;
    QTimer *spinnerTimer = nullptr;

    QPixmap fullScreenPixmap;
    QPixmap currentImage; // 可能是原图，也可能是翻译后的图片
    bool isDragging = false;
    bool isTranslated = false;
    bool isLoading = false;
    bool hasDetectedText = false;
    
    QPoint startPoint;
    QPoint endPoint;
    QRect croppedRect;
    
    FloatingToolbar *toolbar = nullptr;
    NetworkClient *netClient = nullptr;
    ClientConfig config;
};

struct OcrTextItem {
    QRect rect;
    QString text;
};

// 钉图窗口（Pin Window）组件
class PinWindow : public QWidget {
    Q_OBJECT
public:
    explicit PinWindow(const QPixmap &pixmap, const QPoint &pos, QWidget *parent = nullptr);
    ~PinWindow() override;
protected:
    void paintEvent(QPaintEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void contextMenuEvent(QContextMenuEvent *event) override;
    void enterEvent(QEnterEvent *event) override;
    void changeEvent(QEvent *event) override;
private:
    void queryOcr();
    
    QPixmap m_pixmap;
    QPoint m_dragPosition;
    
    QList<OcrTextItem> ocrItems;
    bool isSelectingText = false;
    bool isDraggingWindow = false;
    QPoint selectStartPoint;
    QPoint selectEndPoint;
    
    bool m_hasSelection = false;
    QRect m_selectedRect;
    
    NetworkClient *netClient = nullptr;
    ClientConfig config;
};
