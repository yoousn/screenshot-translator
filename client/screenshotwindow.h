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
#include "localocrmanager.h"
#include "networkclient.h"
#include <QVariantAnimation>
#include <QEasingCurve>

#ifdef Q_OS_WIN
#include <windows.h>
#ifndef WDA_EXCLUDEFROMCAPTURE
#define WDA_EXCLUDEFROMCAPTURE 0x00000011
#endif
#endif

#include <QToolButton>

class VectorToolButton : public QToolButton {
    Q_OBJECT
public:
    enum IconType {
        IconGrip, IconRect, IconCircle, IconArrow, IconPen, IconUndo, IconRedo,
        IconOcr, IconTranslate, IconPin, IconSave, IconSettings, IconClose, IconCopy
    };
    VectorToolButton(IconType type, QWidget *parent = nullptr);
protected:
    void paintEvent(QPaintEvent *event) override;
private:
    IconType m_type;
};

// 悬浮工具栏组件
class FloatingToolbar : public QWidget {
    Q_OBJECT
public:
    explicit FloatingToolbar(QWidget *parent = nullptr);
    void setTranslateEnabled(bool enabled);
    void updateUndoRedo(bool canUndo, bool canRedo);
signals:
    void translateRequested();
    void ocrRequested();
    void copyRequested();
    void saveRequested();
    void pinRequested();
    void settingsRequested();
    void closeRequested();
    
    // Annotation signals
    void toolChanged(int toolType);
    void colorChanged(const QColor &color);
    void widthChanged(int width);
    void undoRequested();
    void redoRequested();
protected:
    void keyPressEvent(QKeyEvent *event) override;
private:
    QWidget *mainBar = nullptr;
    QWidget *styleBar = nullptr;
    VectorToolButton *rectBtn = nullptr;
    VectorToolButton *circleBtn = nullptr;
    VectorToolButton *arrowBtn = nullptr;
    VectorToolButton *penBtn = nullptr;
    VectorToolButton *undoBtn = nullptr;
    VectorToolButton *redoBtn = nullptr;
    VectorToolButton *transBtn = nullptr;
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
    void triggerOcr();
    void copyToClipboard();
    void saveToFile();
    void pinImage();

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
    QRectF currentHighlightRect;
    QRectF targetHighlightRect;
    QVariantAnimation *snapAnimation = nullptr;
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
    
    // Annotation layer properties
    enum AnnotationType { AnnotateNone, AnnotateRect, AnnotateCircle, AnnotateArrow, AnnotatePen };
    struct Annotation {
        AnnotationType type = AnnotateNone;
        QRect rect;
        QList<QPoint> points;
        QColor color = QColor(229, 62, 62);
        int width = 3;
    };
    
    AnnotationType activeAnnotationTool = AnnotateNone;
    QColor activeAnnotationColor = QColor(229, 62, 62);
    int activeAnnotationWidth = 3;
    QList<Annotation> annotations;
    QList<Annotation> undoHistory;
    Annotation currentDraftAnnotation;
    bool isDrawingAnnotation = false;
    
    void drawArrow(QPainter &painter, const QPoint &start, const QPoint &end, const QColor &color, int width);

    FloatingToolbar *toolbar = nullptr;
    NetworkClient *netClient = nullptr;
    LocalOcrManager *localOcrManager = nullptr;
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
    void show() {
        setAttribute(Qt::WA_DeleteOnClose);
        setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
        QWidget::show();
        raise();
        activateWindow();
#ifdef Q_OS_WIN
        QTimer::singleShot(200, this, [this]() {
            if (!isVisible()) return;
            SetWindowDisplayAffinity(reinterpret_cast<HWND>(winId()), WDA_EXCLUDEFROMCAPTURE);
        });
#endif
    }
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
    void applyOcrResults(const QJsonArray &ocrResults);
    void queryRemoteOcr();

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
    LocalOcrManager *localOcrManager = nullptr;
    ClientConfig config;
};
