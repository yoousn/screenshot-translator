#include "screenshotwindow.h"
#include "settingspanel.h"
#include "ocrresultwindow.h"
#include <QApplication>
#include <QScreen>
#include <QPainter>
#include <QMouseEvent>
#include <QKeyEvent>
#include <QClipboard>
#include <QFileDialog>
#include <QMenu>
#include <QKeySequence>
#include <QGraphicsDropShadowEffect>
#include <QPainterPath>
#include <QJsonArray>
#include <QJsonObject>
#include <QShortcut>
#include <QMessageBox>

// ----------------------------------------------------
// 1. VectorToolButton Implementation
// ----------------------------------------------------
VectorToolButton::VectorToolButton(IconType type, QWidget *parent) 
    : QToolButton(parent), m_type(type) {
    setFixedSize(30, 30);
    setCursor(Qt::PointingHandCursor);
}

void VectorToolButton::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    
    bool checked = isChecked();
    bool hovered = underMouse();
    
    if (checked) {
        painter.setBrush(QColor(224, 242, 254)); // #e0f2fe
        painter.setPen(QPen(QColor(14, 165, 233), 1));
        painter.drawRoundedRect(rect().adjusted(1, 1, -1, -1), 6, 6);
    } else if (hovered) {
        painter.setBrush(QColor(241, 245, 249)); // #f1f5f9
        painter.setPen(Qt::NoPen);
        painter.drawRoundedRect(rect().adjusted(1, 1, -1, -1), 6, 6);
    }
    
    QColor iconColor = checked ? QColor(2, 132, 199) : QColor(74, 85, 104);
    QPen iconPen(iconColor, 2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
    painter.setPen(iconPen);
    painter.setBrush(Qt::NoBrush);
    
    int w = width();
    int h = height();
    int cx = w / 2;
    int cy = h / 2;
    
    switch (m_type) {
        case IconGrip: {
            painter.setBrush(QColor(148, 163, 184));
            painter.setPen(Qt::NoPen);
            for (int row = -4; row <= 4; row += 4) {
                painter.drawEllipse(cx - 2, cy + row, 2, 2);
                painter.drawEllipse(cx + 2, cy + row, 2, 2);
            }
            break;
        }
        case IconRect: {
            painter.drawRoundedRect(cx - 7, cy - 7, 14, 14, 2, 2);
            break;
        }
        case IconCircle: {
            painter.drawEllipse(cx - 7, cy - 7, 14, 14);
            break;
        }
        case IconArrow: {
            painter.drawLine(cx - 6, cy + 6, cx + 4, cy - 4);
            QPolygonF arrowHead;
            arrowHead << QPointF(cx + 4, cy - 4)
                      << QPointF(cx - 1, cy - 4)
                      << QPointF(cx + 4, cy + 1);
            painter.setBrush(iconColor);
            painter.drawPolygon(arrowHead);
            break;
        }
        case IconPen: {
            painter.save();
            painter.translate(cx, cy);
            painter.rotate(-45);
            painter.drawRect(-2, -7, 4, 10);
            QPolygonF tip;
            tip << QPointF(-2, 3) << QPointF(0, 6) << QPointF(2, 3);
            painter.setBrush(iconColor);
            painter.drawPolygon(tip);
            painter.restore();
            break;
        }
        case IconUndo: {
            painter.drawArc(cx - 6, cy - 6, 12, 12, -45 * 16, 270 * 16);
            QPolygonF tip;
            tip << QPointF(cx - 6, cy) << QPointF(cx - 9, cy + 4) << QPointF(cx - 3, cy + 4);
            painter.setBrush(iconColor);
            painter.drawPolygon(tip);
            break;
        }
        case IconRedo: {
            painter.drawArc(cx - 6, cy - 6, 12, 12, 45 * 16, -270 * 16);
            QPolygonF tip;
            tip << QPointF(cx + 6, cy) << QPointF(cx + 9, cy + 4) << QPointF(cx + 3, cy + 4);
            painter.setBrush(iconColor);
            painter.drawPolygon(tip);
            break;
        }
        case IconOcr: {
            iconPen.setStyle(Qt::DashLine);
            iconPen.setWidthF(1.5);
            painter.setPen(iconPen);
            painter.drawRect(cx - 8, cy - 8, 16, 16);
            painter.setPen(QPen(iconColor, 1));
            painter.setFont(QFont("Segoe UI", 6, QFont::Bold));
            painter.drawText(rect(), Qt::AlignCenter, "OCR");
            break;
        }
        case IconTranslate: {
            painter.drawEllipse(cx - 7, cy - 7, 14, 14);
            painter.drawLine(cx - 7, cy, cx + 7, cy);
            painter.drawLine(cx, cy - 7, cx, cy + 7);
            break;
        }
        case IconPin: {
            painter.save();
            painter.translate(cx, cy);
            painter.rotate(30);
            painter.drawLine(0, -6, 0, 8);
            painter.fillRect(-4, -6, 8, 3, iconColor);
            painter.fillRect(-2, -3, 4, 5, iconColor);
            painter.restore();
            break;
        }
        case IconSave: {
            painter.drawRect(cx - 7, cy - 7, 14, 14);
            painter.fillRect(cx - 4, cy - 7, 8, 3, iconColor);
            painter.fillRect(cx - 3, cy + 2, 6, 5, iconColor);
            break;
        }
        case IconSettings: {
            painter.drawEllipse(cx - 3, cy - 3, 6, 6);
            for (int i = 0; i < 8; ++i) {
                painter.save();
                painter.translate(cx, cy);
                painter.rotate(i * 45);
                painter.fillRect(-1.5, -8, 3, 3, iconColor);
                painter.restore();
            }
            break;
        }
        case IconClose: {
            painter.drawLine(cx - 5, cy - 5, cx + 5, cy + 5);
            painter.drawLine(cx + 5, cy - 5, cx - 5, cy + 5);
            break;
        }
        case IconCopy: {
            painter.drawRect(cx - 6, cy - 3, 8, 8);
            painter.fillRect(cx - 3, cy - 6, 8, 8, QColor(255, 255, 255, 180));
            QPen pen2(iconColor, 2);
            painter.setPen(pen2);
            painter.drawRect(cx - 3, cy - 6, 8, 8);
            break;
        }
    }
}

// ----------------------------------------------------
// 2. FloatingToolbar Implementation
// ----------------------------------------------------
FloatingToolbar::FloatingToolbar(QWidget *parent) : QWidget(parent) {
    if (!parent) {
        setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    }
    setAttribute(Qt::WA_TranslucentBackground);
    
    QVBoxLayout *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(6, 6, 6, 6);
    mainLayout->setSpacing(6);
    
    // 1. Main Toolbar
    mainBar = new QWidget(this);
    mainBar->setObjectName("mainBar");
    mainBar->setStyleSheet(
        "QWidget#mainBar {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 20px;"
        "}"
    );
    
    // Shadow
    QGraphicsDropShadowEffect *shadow1 = new QGraphicsDropShadowEffect(this);
    shadow1->setBlurRadius(12);
    shadow1->setColor(QColor(0, 0, 0, 20));
    shadow1->setOffset(0, 3);
    mainBar->setGraphicsEffect(shadow1);
    
    QHBoxLayout *barLayout = new QHBoxLayout(mainBar);
    barLayout->setContentsMargins(8, 4, 8, 4);
    barLayout->setSpacing(4);
    
    // Grip
    VectorToolButton *grip = new VectorToolButton(VectorToolButton::IconGrip, mainBar);
    grip->setEnabled(false);
    barLayout->addWidget(grip);
    
    // Shape tools (Toggleable)
    rectBtn = new VectorToolButton(VectorToolButton::IconRect, mainBar);
    rectBtn->setCheckable(true);
    rectBtn->setToolTip("矩形标注");
    barLayout->addWidget(rectBtn);
    
    circleBtn = new VectorToolButton(VectorToolButton::IconCircle, mainBar);
    circleBtn->setCheckable(true);
    circleBtn->setToolTip("椭圆标注");
    barLayout->addWidget(circleBtn);
    
    arrowBtn = new VectorToolButton(VectorToolButton::IconArrow, mainBar);
    arrowBtn->setCheckable(true);
    arrowBtn->setToolTip("箭头标注");
    barLayout->addWidget(arrowBtn);
    
    penBtn = new VectorToolButton(VectorToolButton::IconPen, mainBar);
    penBtn->setCheckable(true);
    penBtn->setToolTip("画笔标注");
    barLayout->addWidget(penBtn);
    
    // Separator 1
    QFrame *sep1 = new QFrame(mainBar);
    sep1->setFrameShape(QFrame::VLine);
    sep1->setFrameShadow(QFrame::Sunken);
    sep1->setStyleSheet("color: #e2e8f0; max-height: 16px; margin: 0 4px;");
    barLayout->addWidget(sep1);
    
    // Undo/Redo
    undoBtn = new VectorToolButton(VectorToolButton::IconUndo, mainBar);
    undoBtn->setToolTip("撤销标注");
    undoBtn->setEnabled(false);
    barLayout->addWidget(undoBtn);
    
    redoBtn = new VectorToolButton(VectorToolButton::IconRedo, mainBar);
    redoBtn->setToolTip("重做标注");
    redoBtn->setEnabled(false);
    barLayout->addWidget(redoBtn);
    
    // Separator 2
    QFrame *sep2 = new QFrame(mainBar);
    sep2->setFrameShape(QFrame::VLine);
    sep2->setFrameShadow(QFrame::Sunken);
    sep2->setStyleSheet("color: #e2e8f0; max-height: 16px; margin: 0 4px;");
    barLayout->addWidget(sep2);
    
    // OCR & Translate
    VectorToolButton *ocrBtn = new VectorToolButton(VectorToolButton::IconOcr, mainBar);
    ocrBtn->setToolTip("识别文字");
    barLayout->addWidget(ocrBtn);
    
    transBtn = new VectorToolButton(VectorToolButton::IconTranslate, mainBar);
    transBtn->setToolTip("截图翻译并原地嵌字 (Ctrl+Q)");
    barLayout->addWidget(transBtn);
    
    // Action tools
    VectorToolButton *pinBtn = new VectorToolButton(VectorToolButton::IconPin, mainBar);
    pinBtn->setToolTip("固定到屏幕 (钉图)");
    barLayout->addWidget(pinBtn);
    
    VectorToolButton *saveBtn = new VectorToolButton(VectorToolButton::IconSave, mainBar);
    saveBtn->setToolTip("保存到本地 (Ctrl+S)");
    barLayout->addWidget(saveBtn);
    
    VectorToolButton *settingsBtn = new VectorToolButton(VectorToolButton::IconSettings, mainBar);
    settingsBtn->setToolTip("选项设置");
    barLayout->addWidget(settingsBtn);
    
    VectorToolButton *closeBtn = new VectorToolButton(VectorToolButton::IconClose, mainBar);
    closeBtn->setToolTip("取消截图 (Esc)");
    barLayout->addWidget(closeBtn);
    
    VectorToolButton *copyBtn = new VectorToolButton(VectorToolButton::IconCopy, mainBar);
    copyBtn->setToolTip("复制截图到剪贴板 (Ctrl+C)");
    barLayout->addWidget(copyBtn);
    
    mainLayout->addWidget(mainBar);
    
    // 2. Styling Sub-toolbar
    styleBar = new QWidget(this);
    styleBar->setObjectName("styleBar");
    styleBar->setStyleSheet(
        "QWidget#styleBar {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 17px;"
        "}"
    );
    styleBar->setVisible(false);
    
    QGraphicsDropShadowEffect *shadow2 = new QGraphicsDropShadowEffect(this);
    shadow2->setBlurRadius(10);
    shadow2->setColor(QColor(0, 0, 0, 15));
    shadow2->setOffset(0, 2);
    styleBar->setGraphicsEffect(shadow2);
    
    QHBoxLayout *styleLayout = new QHBoxLayout(styleBar);
    styleLayout->setContentsMargins(12, 3, 12, 3);
    styleLayout->setSpacing(6);
    
    // 5 Circular Colors
    QList<QColor> colors = {QColor(229, 62, 62), QColor(59, 130, 246), QColor(16, 185, 129), QColor(245, 158, 11), QColor(255, 255, 255)};
    QList<QString> colorNames = {"#e53e3e", "#3b82f6", "#10b981", "#f59e0b", "#ffffff"};
    QList<QToolButton*> colorButtons;
    
    for (int i = 0; i < colors.size(); ++i) {
        QToolButton *cb = new QToolButton(styleBar);
        cb->setFixedSize(16, 16);
        cb->setCursor(Qt::PointingHandCursor);
        cb->setStyleSheet(QString("border-radius: 8px; background-color: %1; border: 1px solid %2;")
                          .arg(colorNames[i]).arg(i == 4 ? "#cbd5e0" : colorNames[i]));
        
        connect(cb, &QToolButton::clicked, [=, &colorButtons]() {
            emit colorChanged(colors[i]);
            // Highlight selected
            for (auto *btn : colorButtons) {
                btn->setDown(false);
                btn->setStyleSheet(btn->styleSheet().replace("border: 2px solid #1a202c;", "border: 1px solid #cbd5e0;"));
            }
            cb->setStyleSheet(cb->styleSheet().replace("border: 1px solid #cbd5e0;", "border: 2px solid #1a202c;"));
        });
        
        colorButtons.append(cb);
        styleLayout->addWidget(cb);
    }
    
    // Select first color by default
    colorButtons.first()->setStyleSheet(colorButtons.first()->styleSheet().replace("border: 1px solid #cbd5e0;", "border: 2px solid #1a202c;"));
    
    // Separator
    QFrame *sepStyle = new QFrame(styleBar);
    sepStyle->setFrameShape(QFrame::VLine);
    sepStyle->setFrameShadow(QFrame::Sunken);
    sepStyle->setStyleSheet("color: #e2e8f0; max-height: 14px; margin: 0 4px;");
    styleLayout->addWidget(sepStyle);
    
    // 3 Brush sizes: 细, 中, 粗
    QList<int> widths = {2, 4, 6};
    QList<QString> widthLabels = {"细", "中", "粗"};
    QList<QToolButton*> widthButtons;
    
    for (int i = 0; i < widths.size(); ++i) {
        QToolButton *wb = new QToolButton(styleBar);
        wb->setFixedSize(24, 20);
        wb->setText(widthLabels[i]);
        wb->setCursor(Qt::PointingHandCursor);
        wb->setStyleSheet(
            "QToolButton {"
            "  background-color: transparent;"
            "  color: #4a5568;"
            "  border: none;"
            "  font-family: 'Segoe UI', 'Microsoft YaHei';"
            "  font-size: 11px;"
            "  border-radius: 4px;"
            "}"
            "QToolButton:hover {"
            "  background-color: #f1f5f9;"
            "}"
        );
        
        connect(wb, &QToolButton::clicked, [=, &widthButtons]() {
            emit widthChanged(widths[i]);
            for (auto *btn : widthButtons) {
                btn->setStyleSheet(btn->styleSheet().replace("background-color: #e0f2fe; color: #0284c7;", "background-color: transparent; color: #4a5568;"));
            }
            wb->setStyleSheet(wb->styleSheet().replace("background-color: transparent; color: #4a5568;", "background-color: #e0f2fe; color: #0284c7;"));
        });
        
        widthButtons.append(wb);
        styleLayout->addWidget(wb);
    }
    
    // Select second width by default
    widthButtons.at(1)->setStyleSheet(widthButtons.at(1)->styleSheet().replace("background-color: transparent; color: #4a5568;", "background-color: #e0f2fe; color: #0284c7;"));
    
    mainLayout->addWidget(styleBar);
    
    // Dynamic shapes show/hide styleBar
    auto updateToolSelection = [=](VectorToolButton *selectedBtn, int toolType) {
        // Toggle action
        if (selectedBtn->isChecked()) {
            // Uncheck other buttons
            if (selectedBtn != rectBtn) rectBtn->setChecked(false);
            if (selectedBtn != circleBtn) circleBtn->setChecked(false);
            if (selectedBtn != arrowBtn) arrowBtn->setChecked(false);
            if (selectedBtn != penBtn) penBtn->setChecked(false);
            
            styleBar->setVisible(true);
            emit toolChanged(toolType);
        } else {
            styleBar->setVisible(false);
            emit toolChanged(0); // AnnotateNone
        }
        adjustSize();
    };
    
    connect(rectBtn, &QToolButton::clicked, [=]() { updateToolSelection(rectBtn, 1); });
    connect(circleBtn, &QToolButton::clicked, [=]() { updateToolSelection(circleBtn, 2); });
    connect(arrowBtn, &QToolButton::clicked, [=]() { updateToolSelection(arrowBtn, 3); });
    connect(penBtn, &QToolButton::clicked, [=]() { updateToolSelection(penBtn, 4); });
    
    // Action connections
    connect(ocrBtn, &QToolButton::clicked, this, &FloatingToolbar::ocrRequested);
    connect(transBtn, &QToolButton::clicked, this, &FloatingToolbar::translateRequested);
    connect(pinBtn, &QToolButton::clicked, this, &FloatingToolbar::pinRequested);
    connect(saveBtn, &QToolButton::clicked, this, &FloatingToolbar::saveRequested);
    connect(settingsBtn, &QToolButton::clicked, this, &FloatingToolbar::settingsRequested);
    connect(closeBtn, &QToolButton::clicked, this, &FloatingToolbar::closeRequested);
    connect(copyBtn, &QToolButton::clicked, this, &FloatingToolbar::copyRequested);
    
    connect(undoBtn, &QToolButton::clicked, this, &FloatingToolbar::undoRequested);
    connect(redoBtn, &QToolButton::clicked, this, &FloatingToolbar::redoRequested);
    
    adjustSize();
}

void FloatingToolbar::setTranslateEnabled(bool enabled) {
    if (transBtn) transBtn->setEnabled(enabled);
}

void FloatingToolbar::updateUndoRedo(bool canUndo, bool canRedo) {
    if (undoBtn) undoBtn->setEnabled(canUndo);
    if (redoBtn) redoBtn->setEnabled(canRedo);
}

void FloatingToolbar::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Escape) {
        emit closeRequested();
    } else {
        QWidget::keyPressEvent(event);
    }
}

// ----------------------------------------------------
// 2. ScreenshotWindow Implementation
// ----------------------------------------------------
#ifdef Q_OS_WIN
#include <windows.h>

struct EnumWindowsData {
    QList<QRect> *rects;
    HWND currentHwnd;
    qreal dpiRatio;
};

BOOL CALLBACK EnumWindowsProc(HWND hwnd, LPARAM lParam) {
    EnumWindowsData *data = reinterpret_cast<EnumWindowsData*>(lParam);
    if (hwnd == data->currentHwnd) return TRUE;
    
    // 1. Must be visible and not minimized
    if (!IsWindowVisible(hwnd)) return TRUE;
    if (IsIconic(hwnd)) return TRUE;
    
    // 2. Must be a top-level window (no parent)
    if (GetParent(hwnd) != NULL) return TRUE;
    
    // 3. Must have a title to prevent snapping to background overlays/invisible helper windows
    int titleLen = GetWindowTextLengthW(hwnd);
    if (titleLen == 0) return TRUE;
    
    // 4. Ignore cloaked windows (Universal background apps, suspended UWP apps, etc.)
    int cloaked = 0;
    typedef HRESULT (WINAPI *pfnDwmGetWindowAttribute)(HWND, DWORD, PVOID, DWORD);
    static pfnDwmGetWindowAttribute dwmGetWindowAttribute = nullptr;
    static bool dwmLoaded = false;
    if (!dwmLoaded) {
        HMODULE hDwmapi = LoadLibraryW(L"dwmapi.dll");
        if (hDwmapi) {
            dwmGetWindowAttribute = (pfnDwmGetWindowAttribute)GetProcAddress(hDwmapi, "DwmGetWindowAttribute");
        }
        dwmLoaded = true;
    }
    if (dwmGetWindowAttribute) {
        dwmGetWindowAttribute(hwnd, 14, &cloaked, sizeof(cloaked)); // 14 = DWMWA_CLOAKED
        if (cloaked) return TRUE;
    }
    
    LONG_PTR style = GetWindowLongPtr(hwnd, GWL_STYLE);
    LONG_PTR exStyle = GetWindowLongPtr(hwnd, GWL_EXSTYLE);
    
    // 5. Must not be a tool window
    if (exStyle & WS_EX_TOOLWINDOW) return TRUE;
    
    // 6. If it has an owner, it must have the WS_EX_APPWINDOW style to be a real app window
    HWND owner = GetWindow(hwnd, GW_OWNER);
    if (owner != NULL && !(exStyle & WS_EX_APPWINDOW)) return TRUE;
    
    RECT r;
    bool gotRect = false;
    if (dwmGetWindowAttribute) {
        // 9 = DWMWA_EXTENDED_FRAME_BOUNDS
        HRESULT hr = dwmGetWindowAttribute(hwnd, 9, &r, sizeof(r));
        if (SUCCEEDED(hr)) gotRect = true;
    }
    if (!gotRect) {
        if (!GetWindowRect(hwnd, &r)) return TRUE;
    }
    
    int w = r.right - r.left;
    int h = r.bottom - r.top;
    if (w > 100 && h > 100) { // Filter out small popups, tooltips, etc.
        qreal ratio = data->dpiRatio;
        int lx = r.left / ratio;
        int ly = r.top / ratio;
        int lw = w / ratio;
        int lh = h / ratio;
        data->rects->append(QRect(lx, ly, lw, lh));
    }
    return TRUE;
}
#endif

// ----------------------------------------------------
// 2. ScreenshotWindow Implementation
// ----------------------------------------------------
ScreenshotWindow::ScreenshotWindow(QWidget *parent) : QWidget(parent) {
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_DeleteOnClose);
    setFocusPolicy(Qt::StrongFocus);
    
    config.load();
    netClient = new NetworkClient(this);
    localOcrManager = new LocalOcrManager(this);
    
    // Support ESC to close the screenshot window bulletproofly
    QShortcut *escShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    connect(escShortcut, &QShortcut::activated, this, &QWidget::close);
    
    // 抓取全屏
    QScreen *screen = QApplication::primaryScreen();
    qreal ratio = 1.0;
    if (screen) {
        fullScreenPixmap = screen->grabWindow(0);
        ratio = screen->devicePixelRatio();
    }
    
    fullScreenImage = fullScreenPixmap.toImage();
    
    // 初始化旋转加载定时器（仅在 loading 时启动）
    spinnerTimer = new QTimer(this);
    connect(spinnerTimer, &QTimer::timeout, this, [=]() {
        spinnerAngle = (spinnerAngle + 12) % 360;
        update();
    });

    // 初始化窗口边缘吸附平滑动画
    snapAnimation = new QVariantAnimation(this);
    snapAnimation->setDuration(180);
    snapAnimation->setEasingCurve(QEasingCurve::OutCubic);
    connect(snapAnimation, &QVariantAnimation::valueChanged, this, [this](const QVariant &val) {
        currentHighlightRect = val.toRectF();
        update();
    });
    
    // 窗口探测 (Win32 API)
#ifdef Q_OS_WIN
    EnumWindowsData data;
    data.rects = &detectedWindowRects;
    data.currentHwnd = (HWND)this->winId();
    data.dpiRatio = ratio;
    EnumWindows((WNDENUMPROC)EnumWindowsProc, (LPARAM)&data);
#endif

    showFullScreen();
    setCursor(Qt::CrossCursor);
    setMouseTracking(true);
}

ScreenshotWindow::~ScreenshotWindow() {
    if (spinnerTimer) {
        spinnerTimer->stop();
    }
    hideToolbar();
}

void ScreenshotWindow::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.drawPixmap(0, 0, fullScreenPixmap);
    
    // 半透明黑色遮罩
    painter.fillRect(rect(), QColor(0, 0, 0, 110));
    
    // 悬停自动窗口边界高亮
    if (croppedRect.isEmpty() && !currentHighlightRect.isEmpty()) {
        QRect highlightRect = currentHighlightRect.toAlignedRect();
        painter.drawPixmap(highlightRect, fullScreenPixmap, highlightRect);
        painter.setPen(QPen(QColor(24, 144, 255), 2, Qt::DashLine));
        painter.drawRect(currentHighlightRect);
    }
    
    if (isDragging || !croppedRect.isEmpty()) {
        // 重新照亮选区
        painter.drawPixmap(croppedRect, fullScreenPixmap, croppedRect);
        
        // 如果已经返回了翻译结果图，将其绘制覆盖于选区上
        if (isTranslated && !currentImage.isNull()) {
            painter.drawPixmap(croppedRect, currentImage);
        }
        
        // ── 绘制标注图层 ──
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setClipRect(croppedRect); // 剪切区域，确保画笔不溢出截图框
        
        for (const auto &ann : annotations) {
            QPen p(ann.color, ann.width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
            painter.setPen(p);
            painter.setBrush(Qt::NoBrush);
            switch (ann.type) {
                case AnnotateRect:
                    painter.drawRect(ann.rect);
                    break;
                case AnnotateCircle:
                    painter.drawEllipse(ann.rect);
                    break;
                case AnnotateArrow:
                    drawArrow(painter, ann.rect.topLeft(), ann.rect.bottomRight(), ann.color, ann.width);
                    break;
                case AnnotatePen: {
                    for (int i = 1; i < ann.points.size(); ++i) {
                        painter.drawLine(ann.points[i - 1], ann.points[i]);
                    }
                    break;
                }
                default: break;
            }
        }
        
        // 绘制正在草拟的当前标注
        if (isDrawingAnnotation) {
            QPen p(currentDraftAnnotation.color, currentDraftAnnotation.width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
            painter.setPen(p);
            painter.setBrush(Qt::NoBrush);
            switch (currentDraftAnnotation.type) {
                case AnnotateRect:
                    painter.drawRect(currentDraftAnnotation.rect);
                    break;
                case AnnotateCircle:
                    painter.drawEllipse(currentDraftAnnotation.rect);
                    break;
                case AnnotateArrow:
                    drawArrow(painter, currentDraftAnnotation.rect.topLeft(), currentDraftAnnotation.rect.bottomRight(), currentDraftAnnotation.color, currentDraftAnnotation.width);
                    break;
                case AnnotatePen: {
                    for (int i = 1; i < currentDraftAnnotation.points.size(); ++i) {
                        painter.drawLine(currentDraftAnnotation.points[i - 1], currentDraftAnnotation.points[i]);
                    }
                    break;
                }
                default: break;
            }
        }
        painter.restore();
        
        // 绘制选区亮蓝色边框
        painter.setPen(QPen(QColor(24, 144, 255), 2));
        painter.drawRect(croppedRect);
        
        // 绘制选区的大小指示器 (使用背景药丸框提升对比度)
        QString sizeText = QString("%1 x %2").arg(croppedRect.width()).arg(croppedRect.height());
        painter.setPen(Qt::white);
        painter.setFont(QFont("Segoe UI", 9));
        
        int tw = painter.fontMetrics().horizontalAdvance(sizeText);
        int th = painter.fontMetrics().height();
        QRect sizeRect(croppedRect.left() + 4, croppedRect.top() - th - 8, tw + 8, th + 4);
        if (sizeRect.top() < 0) {
            sizeRect.moveTop(croppedRect.top() + 4);
        }
        painter.fillRect(sizeRect, QColor(0, 0, 0, 160));
        painter.drawText(sizeRect, Qt::AlignCenter, sizeText);
        
        // 只有在非拖拽状态且不是正在翻译时，绘制 8 个缩放控制手柄
        if (!isDragging && captureState == StateIdle && !isLoading) {
            const int handleRadius = 4;
            QList<QPoint> handlePoints = getHandlePoints();
            painter.setRenderHint(QPainter::Antialiasing, true);
            for (const QPoint &pt : handlePoints) {
                painter.setPen(QPen(QColor(24, 144, 255), 2));
                painter.setBrush(Qt::white);
                painter.drawEllipse(pt, handleRadius, handleRadius);
            }
        }
        
        // 正在翻译的加载中状态 (旋转加载指示器)
        if (isLoading) {
            painter.fillRect(croppedRect, QColor(0, 0, 0, 140));
            painter.setRenderHint(QPainter::Antialiasing, true);
            
            painter.setPen(QPen(QColor(24, 144, 255), 3, Qt::SolidLine, Qt::RoundCap));
            int size = qMin(40, qMin(croppedRect.width(), croppedRect.height()) / 2);
            if (size > 10) {
                QRect arcRect(croppedRect.center().x() - size/2, croppedRect.center().y() - size/2, size, size);
                painter.drawArc(arcRect, spinnerAngle * 16, 280 * 16);
            }
        }
    }
    
    // 未选择状态下显示鼠标追踪指示器 (坐标 & RGB 拾色器 HUD)
    if (!isDragging && croppedRect.isEmpty()) {
        drawHUD(painter);
    }
}

void ScreenshotWindow::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::RightButton) {
        if (!croppedRect.isEmpty()) {
            croppedRect = QRect();
            annotations.clear();
            undoHistory.clear();
            isTranslated = false;
            activeAnnotationTool = AnnotateNone;
            isDrawingAnnotation = false;
            hideToolbar();
            update();
        } else {
            close();
        }
        return;
    }

    if (event->button() == Qt::LeftButton) {
        pressPos = event->pos();
        
        // If drawing annotation tool is active, handle drawing instead of cropping!
        if (activeAnnotationTool != AnnotateNone) {
            if (!croppedRect.isEmpty() && croppedRect.contains(event->pos())) {
                isDrawingAnnotation = true;
                currentDraftAnnotation.type = activeAnnotationTool;
                currentDraftAnnotation.color = activeAnnotationColor;
                currentDraftAnnotation.width = activeAnnotationWidth;
                currentDraftAnnotation.rect = QRect(event->pos(), event->pos());
                currentDraftAnnotation.points.clear();
                currentDraftAnnotation.points.append(event->pos());
                update();
                return;
            }
        }
        
        int handle = getHandleAt(event->pos());
        if (handle >= 0) {
            captureState = StateResizing;
            activeHandle = handle;
            isDragging = true;
            hideToolbar();
            if (isTranslated) isTranslated = false;
        } else if (!croppedRect.isEmpty() && croppedRect.contains(event->pos())) {
            captureState = StateMoving;
            dragOffset = event->pos() - croppedRect.topLeft();
            isDragging = true;
            hideToolbar();
            if (isTranslated) isTranslated = false;
        } else {
            // Clicked outside selection! Fulfill Requirement 4!
            if (!croppedRect.isEmpty()) {
                copyToClipboard();
                return;
            }
            captureState = StateSelecting;
            isDragging = true;
            hideToolbar();
            startPoint = event->pos();
            endPoint = startPoint;
            
            if (isTranslated) {
                isTranslated = false;
            }
        }
        update();
    }
}

void ScreenshotWindow::mouseMoveEvent(QMouseEvent *event) {
    currentMousePos = event->pos();
    QPoint globalPos = mapToGlobal(currentMousePos);
    
    if (currentMousePos.x() >= 0 && currentMousePos.x() < fullScreenImage.width() &&
        currentMousePos.y() >= 0 && currentMousePos.y() < fullScreenImage.height()) {
        currentColor = fullScreenImage.pixelColor(currentMousePos);
    } else {
        currentColor = Qt::black;
    }
    
    if (isDrawingAnnotation) {
        if (activeAnnotationTool == AnnotatePen) {
            currentDraftAnnotation.points.append(event->pos());
        } else {
            currentDraftAnnotation.rect = QRect(pressPos, event->pos());
        }
        update();
        return;
    }
    
    if (isDragging) {
        if (captureState == StateSelecting) {
            endPoint = event->pos();
            croppedRect = QRect(startPoint, endPoint).normalized();
        } else if (captureState == StateMoving) {
            QPoint newTopLeft = event->pos() - dragOffset;
            int w = croppedRect.width();
            int h = croppedRect.height();
            int x = qMax(0, qMin(newTopLeft.x(), width() - w));
            int y = qMax(0, qMin(newTopLeft.y(), height() - h));
            croppedRect = QRect(x, y, w, h);
        } else if (captureState == StateResizing) {
            resizeSelection(event->pos());
        }
        update();
    } else {
        updateCursorShape(event->pos());
        if (croppedRect.isEmpty()) {
            QRect snap = getSnappedRect(globalPos);
            QRect target = QRect(mapFromGlobal(snap.topLeft()), snap.size()).intersected(rect());
            if (target != targetHighlightRect) {
                targetHighlightRect = target;
                snapAnimation->stop();
                snapAnimation->setStartValue(currentHighlightRect.isEmpty() ? QRectF(target) : currentHighlightRect);
                snapAnimation->setEndValue(QRectF(target));
                snapAnimation->start();
            }
        } else {
            targetHighlightRect = QRectF();
            currentHighlightRect = QRectF();
            snapAnimation->stop();
        }
    }
}

void ScreenshotWindow::mouseReleaseEvent(QMouseEvent *event) {
    if (isDrawingAnnotation) {
        isDrawingAnnotation = false;
        bool valid = false;
        if (currentDraftAnnotation.type == AnnotatePen) {
            valid = currentDraftAnnotation.points.size() > 1;
        } else {
            valid = currentDraftAnnotation.rect.width() > 2 || currentDraftAnnotation.rect.height() > 2;
        }
        if (valid) {
            annotations.append(currentDraftAnnotation);
            undoHistory.clear(); // Clear redo stack on new action
            if (toolbar) toolbar->updateUndoRedo(true, false);
        }
        update();
        return;
    }

    if (event->button() == Qt::LeftButton && isDragging) {
        isDragging = false;
        
        if (captureState == StateSelecting) {
            int dist = (event->pos() - pressPos).manhattanLength();
            if (dist < 6) {
                QRect targetRect = targetHighlightRect.toAlignedRect();
                if (!targetRect.isEmpty() && targetRect.width() > 10 && targetRect.height() > 10) {
                    croppedRect = targetRect;
                } else {
                    croppedRect = QRect();
                }
            }
        }
        
        captureState = StateIdle;
        
        if (croppedRect.width() > 10 && croppedRect.height() > 10) {
            currentImage = fullScreenPixmap.copy(croppedRect);
            currentImage.setDevicePixelRatio(devicePixelRatio());
            showToolbar();
        } else {
            croppedRect = QRect();
            hideToolbar();
        }
        update();
    }
}

void ScreenshotWindow::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Escape) {
        close();
    } else if (event->modifiers() == Qt::ControlModifier && event->key() == Qt::Key_Q) {
        if (!croppedRect.isEmpty()) triggerTranslation();
    } else if (event->modifiers() == Qt::ControlModifier && event->key() == Qt::Key_C) {
        if (!croppedRect.isEmpty()) copyToClipboard();
    } else if (event->modifiers() == Qt::ControlModifier && event->key() == Qt::Key_S) {
        if (!croppedRect.isEmpty()) saveToFile();
    } else if (event->modifiers() == Qt::ControlModifier && event->key() == Qt::Key_Z) {
        if (!annotations.isEmpty()) {
            undoHistory.append(annotations.takeLast());
            update();
            if (toolbar) toolbar->updateUndoRedo(!annotations.isEmpty(), !undoHistory.isEmpty());
        }
    } else if (event->modifiers() == Qt::ControlModifier && event->key() == Qt::Key_Y) {
        if (!undoHistory.isEmpty()) {
            annotations.append(undoHistory.takeLast());
            update();
            if (toolbar) toolbar->updateUndoRedo(!annotations.isEmpty(), !undoHistory.isEmpty());
        }
    } else {
        QWidget::keyPressEvent(event);
    }
}

QList<QPoint> ScreenshotWindow::getHandlePoints() const {
    QList<QPoint> points;
    if (croppedRect.isEmpty()) return points;
    
    int l = croppedRect.left();
    int r = croppedRect.right();
    int t = croppedRect.top();
    int b = croppedRect.bottom();
    int hc = croppedRect.center().x();
    int vc = croppedRect.center().y();
    
    points << QPoint(l, t)    // 0: Top-Left
           << QPoint(hc, t)   // 1: Top-Center
           << QPoint(r, t)    // 2: Top-Right
           << QPoint(r, vc)   // 3: Middle-Right
           << QPoint(r, b)    // 4: Bottom-Right
           << QPoint(hc, b)   // 5: Bottom-Center
           << QPoint(l, b)    // 6: Bottom-Left
           << QPoint(l, vc);  // 7: Middle-Left
           
    return points;
}

int ScreenshotWindow::getHandleAt(const QPoint &pos) {
    if (croppedRect.isEmpty()) return -1;
    const int threshold = 8;
    QList<QPoint> points = getHandlePoints();
    for (int i = 0; i < points.size(); ++i) {
        QPoint diff = pos - points[i];
        if (diff.x() * diff.x() + diff.y() * diff.y() <= threshold * threshold) {
            return i;
        }
    }
    return -1;
}

void ScreenshotWindow::resizeSelection(const QPoint &pos) {
    int l = croppedRect.left();
    int r = croppedRect.right();
    int t = croppedRect.top();
    int b = croppedRect.bottom();
    
    switch (activeHandle) {
        case 0: // Top-Left
            l = pos.x();
            t = pos.y();
            break;
        case 1: // Top-Center
            t = pos.y();
            break;
        case 2: // Top-Right
            r = pos.x();
            t = pos.y();
            break;
        case 3: // Middle-Right
            r = pos.x();
            break;
        case 4: // Bottom-Right
            r = pos.x();
            b = pos.y();
            break;
        case 5: // Bottom-Center
            b = pos.y();
            break;
        case 6: // Bottom-Left
            l = pos.x();
            b = pos.y();
            break;
        case 7: // Middle-Left
            l = pos.x();
            break;
    }
    
    croppedRect = QRect(QPoint(l, t), QPoint(r, b)).normalized();
}

void ScreenshotWindow::updateCursorShape(const QPoint &pos) {
    if (croppedRect.isEmpty()) {
        setCursor(Qt::CrossCursor);
        return;
    }
    if (activeAnnotationTool != AnnotateNone) {
        if (croppedRect.contains(pos)) {
            setCursor(Qt::CrossCursor);
        } else {
            setCursor(Qt::ArrowCursor);
        }
        return;
    }
    int handle = getHandleAt(pos);
    if (handle >= 0) {
        if (handle == 0 || handle == 4) {
            setCursor(Qt::SizeFDiagCursor);
        } else if (handle == 2 || handle == 6) {
            setCursor(Qt::SizeBDiagCursor);
        } else if (handle == 1 || handle == 5) {
            setCursor(Qt::SizeVerCursor);
        } else if (handle == 3 || handle == 7) {
            setCursor(Qt::SizeHorCursor);
        }
    } else if (croppedRect.contains(pos)) {
        setCursor(Qt::SizeAllCursor);
    } else {
        setCursor(Qt::CrossCursor);
    }
}

QRect ScreenshotWindow::getSnappedRect(const QPoint &pos) {
    QRect bestRect;
    int minArea = std::numeric_limits<int>::max();
    for (const QRect &rect : detectedWindowRects) {
        if (rect.contains(pos)) {
            int area = rect.width() * rect.height();
            if (area < minArea) {
                minArea = area;
                bestRect = rect;
            }
        }
    }
    return bestRect;
}

void ScreenshotWindow::drawHUD(QPainter &painter) {
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    
    int hudW = 150;
    int hudH = 75;
    int x = currentMousePos.x() + 15;
    int y = currentMousePos.y() + 15;
    
    if (x + hudW > width()) x = currentMousePos.x() - hudW - 15;
    if (y + hudH > height()) y = currentMousePos.y() - hudH - 15;
    
    QRect hudRect(x, y, hudW, hudH);
    
    // 背景卡片
    painter.fillRect(hudRect, QColor(0, 0, 0, 180));
    painter.setPen(QPen(QColor(255, 255, 255, 60), 1));
    painter.drawRect(hudRect);
    
    // 颜色方框
    QRect colorBox(x + 8, y + 8, 20, 20);
    painter.fillRect(colorBox, currentColor);
    painter.setPen(Qt::white);
    painter.drawRect(colorBox);
    
    // 坐标
    QString posText = QString("%1, %2").arg(currentMousePos.x()).arg(currentMousePos.y());
    painter.setFont(QFont("Segoe UI", 9, QFont::Bold));
    painter.setPen(Qt::white);
    painter.drawText(x + 36, y + 22, posText);
    
    // RGB & HEX 文字
    QString rgbText = QString("RGB: (%1, %2, %3)").arg(currentColor.red()).arg(currentColor.green()).arg(currentColor.blue());
    QString hexText = QString("HEX: %1").arg(currentColor.name().toUpper());
    
    painter.setFont(QFont("Segoe UI", 8));
    painter.setPen(QColor(220, 220, 220));
    painter.drawText(x + 8, y + 46, rgbText);
    painter.drawText(x + 8, y + 62, hexText);
    
    painter.restore();
}

void ScreenshotWindow::drawArrow(QPainter &painter, const QPoint &start, const QPoint &end, const QColor &color, int width) {
    painter.save();
    painter.setPen(QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawLine(start, end);
    
    double angle = std::atan2(end.y() - start.y(), end.x() - start.x());
    qreal headLength = 12 + width * 1.5;
    QPointF p1 = end - QPointF(std::cos(angle + M_PI/6) * headLength, std::sin(angle + M_PI/6) * headLength);
    QPointF p2 = end - QPointF(std::cos(angle - M_PI/6) * headLength, std::sin(angle - M_PI/6) * headLength);
    
    painter.setBrush(color);
    QPolygonF head;
    head << end << p1 << p2;
    painter.drawPolygon(head);
    painter.restore();
}

void ScreenshotWindow::showToolbar() {
    if (!toolbar) {
        toolbar = new FloatingToolbar(this);
        connect(toolbar, &FloatingToolbar::translateRequested, this, &ScreenshotWindow::triggerTranslation);
        connect(toolbar, &FloatingToolbar::ocrRequested, this, &ScreenshotWindow::triggerOcr);
        connect(toolbar, &FloatingToolbar::copyRequested, this, &ScreenshotWindow::copyToClipboard);
        connect(toolbar, &FloatingToolbar::saveRequested, this, &ScreenshotWindow::saveToFile);
        connect(toolbar, &FloatingToolbar::pinRequested, this, &ScreenshotWindow::pinImage);
        connect(toolbar, &FloatingToolbar::settingsRequested, this, [=]() {
            if (!SettingsPanel::activeInstance) {
                SettingsPanel::activeInstance = new SettingsPanel();
            }
            SettingsPanel::activeInstance->show();
            SettingsPanel::activeInstance->raise();
            SettingsPanel::activeInstance->activateWindow();
        });
        connect(toolbar, &FloatingToolbar::closeRequested, this, &ScreenshotWindow::close);
        
        // Connect annotation signals
        connect(toolbar, &FloatingToolbar::toolChanged, this, [this](int toolType) {
            activeAnnotationTool = static_cast<AnnotationType>(toolType);
            if (activeAnnotationTool != AnnotateNone) {
                setCursor(Qt::CrossCursor);
            } else {
                setCursor(Qt::SizeAllCursor);
            }
        });
        connect(toolbar, &FloatingToolbar::colorChanged, this, [this](const QColor &color) {
            activeAnnotationColor = color;
        });
        connect(toolbar, &FloatingToolbar::widthChanged, this, [this](int width) {
            activeAnnotationWidth = width;
        });
        connect(toolbar, &FloatingToolbar::undoRequested, this, [this]() {
            if (!annotations.isEmpty()) {
                undoHistory.append(annotations.takeLast());
                update();
                if (toolbar) toolbar->updateUndoRedo(!annotations.isEmpty(), !undoHistory.isEmpty());
            }
        });
        connect(toolbar, &FloatingToolbar::redoRequested, this, [this]() {
            if (!undoHistory.isEmpty()) {
                annotations.append(undoHistory.takeLast());
                update();
                if (toolbar) toolbar->updateUndoRedo(!annotations.isEmpty(), !undoHistory.isEmpty());
            }
        });
    }
    // 直接允许翻译，无需等待 OCR 检测
    hasDetectedText = true;
    toolbar->setTranslateEnabled(true);
    // Initialize undo/redo state on show
    toolbar->updateUndoRedo(!annotations.isEmpty(), !undoHistory.isEmpty());
    updateToolbarPosition();
    toolbar->show();
}

void ScreenshotWindow::hideToolbar() {
    if (toolbar) {
        toolbar->hide();
        toolbar->deleteLater();
        toolbar = nullptr;
    }
}

void ScreenshotWindow::updateToolbarPosition() {
    if (!toolbar) return;
    
    // 默认定位在选区下方 10 像素
    int x = croppedRect.center().x() - toolbar->width() / 2;
    int y = croppedRect.bottom() + 10;
    
    // 边界碰撞检测
    if (y + toolbar->height() > height()) {
        y = croppedRect.top() - toolbar->height() - 10; // 向上反弹
    }
    if (x < 0) x = 10;
    if (x + toolbar->width() > width()) x = width() - toolbar->width() - 10;
    
    toolbar->move(x, y);
}


void ScreenshotWindow::triggerTranslation() {
    if (isLoading || croppedRect.isEmpty() || !hasDetectedText) return;
    
    isLoading = true;
    if (toolbar) toolbar->setTranslateEnabled(false);
    if (spinnerTimer) spinnerTimer->start(50); // 开始旋转动画
    update();
    
    QPixmap originalCrop = fullScreenPixmap.copy(croppedRect);
    
    netClient->translateImage(originalCrop, config, [=](bool success, const QPixmap &resPixmap) {
        isLoading = false;
        if (spinnerTimer) spinnerTimer->stop(); // 停止旋转动画
        if (success && !resPixmap.isNull()) {
            currentImage = resPixmap;
            currentImage.setDevicePixelRatio(devicePixelRatio());
            isTranslated = true;
            // 翻译成功后自动转为“钉图”固定在屏幕上，支持自由拖动且永不失真退色
            pinImage();
            return;
        }
        if (toolbar) toolbar->setTranslateEnabled(hasDetectedText);
        update();
    });
}

void ScreenshotWindow::triggerOcr() {
    if (isLoading || croppedRect.isEmpty()) return;
    
    isLoading = true;
    if (toolbar) toolbar->setEnabled(false);
    if (spinnerTimer) spinnerTimer->start(50); // 开始旋转动画
    update();
    
    QPixmap originalCrop = fullScreenPixmap.copy(croppedRect);
    
    auto onOcrFinished = [this, originalCrop](bool success, const QJsonArray &ocrResults, const QString &err) {
        isLoading = false;
        if (spinnerTimer) spinnerTimer->stop(); // 停止旋转动画
        if (success) {
            QStringList lines;
            for (const QJsonValue &val : ocrResults) {
                QJsonObject obj = val.toObject();
                lines.append(obj.value("text").toString());
            }
            QString fullText = lines.join("\n");
            
            OcrResultWindow *resultWin = new OcrResultWindow(fullText, originalCrop);
            resultWin->show();
            
            close();
        } else {
            if (toolbar) toolbar->setEnabled(true);
            update();
            QMessageBox::warning(this, "识别失败", "无法识别选区中的文字：" + err);
        }
    };
    
    if (config.useLocalOcr) {
        localOcrManager->ocrImage(originalCrop, config, [=](bool success, const QJsonArray &ocrResults, const QString &err) {
            if (success) {
                onOcrFinished(true, ocrResults, "");
                return;
            }
            if (config.fallbackToRemoteOcr) {
                netClient->ocrImage(originalCrop, config, [=](bool remoteSuccess, const QJsonArray &remoteOcrResults) {
                    onOcrFinished(remoteSuccess, remoteOcrResults, remoteSuccess ? "" : "云端 OCR 失败");
                });
            } else {
                onOcrFinished(false, QJsonArray(), err);
            }
        });
    } else {
        netClient->ocrImage(originalCrop, config, [=](bool remoteSuccess, const QJsonArray &remoteOcrResults) {
            onOcrFinished(remoteSuccess, remoteOcrResults, remoteSuccess ? "" : "云端 OCR 失败");
        });
    }
}

void ScreenshotWindow::copyToClipboard() {
    if (currentImage.isNull()) return;
    QApplication::clipboard()->setPixmap(currentImage);
    close();
}

void ScreenshotWindow::saveToFile() {
    if (currentImage.isNull()) return;
    
    QString filePath = QFileDialog::getSaveFileName(
        this, "保存截图", "screenshot.png", "PNG Image (*.png);;JPEG Image (*.jpg)"
    );
    if (!filePath.isEmpty()) {
        currentImage.save(filePath);
        close();
    }
}

void ScreenshotWindow::pinImage() {
    if (currentImage.isNull()) return;
    
    // 在桌面原位置放置钉图
    PinWindow *pin = new PinWindow(currentImage, croppedRect.topLeft());
    pin->show();
    close();
}

// ----------------------------------------------------
// 3. PinWindow Implementation
// ----------------------------------------------------
PinWindow::PinWindow(const QPixmap &pixmap, const QPoint &pos, QWidget *parent) 
    : QWidget(parent), m_pixmap(pixmap) {
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::SubWindow);
    setAttribute(Qt::WA_TranslucentBackground);
    setFocusPolicy(Qt::StrongFocus);
    
    // We add 10px margin on all sides for the soft glowing shadow effect
    resize(pixmap.width() + 20, pixmap.height() + 20);
    move(pos - QPoint(10, 10));
    
    config.load();
    netClient = new NetworkClient(this);
    localOcrManager = new LocalOcrManager(this);

    // Support ESC to close the pinned window bulletproofly
    QShortcut *escShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    connect(escShortcut, &QShortcut::activated, this, &QWidget::close);
    
    // Asynchronously request OCR on the pinned image in the background
    queryOcr();
}

PinWindow::~PinWindow() {
}

void PinWindow::queryOcr() {
    if (config.useLocalOcr) {
        localOcrManager->ocrImage(m_pixmap, config, [this](bool success, const QJsonArray &ocrResults, const QString &) {
            if (success) {
                applyOcrResults(ocrResults);
                return;
            }
            if (config.fallbackToRemoteOcr) {
                queryRemoteOcr();
            }
        });
        return;
    }

    queryRemoteOcr();
}

void PinWindow::queryRemoteOcr() {
    netClient->ocrImage(m_pixmap, config, [this](bool success, const QJsonArray &ocrResults) {
        if (success) {
            applyOcrResults(ocrResults);
        }
    });
}

void PinWindow::applyOcrResults(const QJsonArray &ocrResults) {
    ocrItems.clear();
    qreal ratio = devicePixelRatio();
    for (const auto &val : ocrResults) {
        QJsonObject obj = val.toObject();
        QJsonArray box = obj.value("box").toArray();
        if (box.size() == 4 && box.at(0).isArray() && box.at(2).isArray()) {
            QJsonArray p1 = box.at(0).toArray();
            QJsonArray p3 = box.at(2).toArray();
            if (p1.size() < 2 || p3.size() < 2) continue;
            int x1 = p1.at(0).toDouble() / ratio;
            int y1 = p1.at(1).toDouble() / ratio;
            int x3 = p3.at(0).toDouble() / ratio;
            int y3 = p3.at(1).toDouble() / ratio;

            OcrTextItem item;
            // Translate coordinates by +10 because the image is painted at (10, 10)
            item.rect = QRect(QPoint(x1, y1), QPoint(x3, y3)).translated(10, 10);
            item.text = obj.value("text").toString();
            ocrItems.append(item);
        }
    }
}

void PinWindow::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    
    // Clear background
    painter.fillRect(rect(), Qt::transparent);
    
    QRect contentRect(10, 10, m_pixmap.width(), m_pixmap.height());
    
    // Check if the window is currently active (focused)
    bool active = isActiveWindow();
    QColor glowColor = active ? QColor(52, 152, 219) : QColor(120, 120, 120);
    int maxAlpha = active ? 40 : 15;
    
    // Draw the soft glow around contentRect
    for (int i = 0; i < 10; ++i) {
        QPainterPath path;
        path.addRoundedRect(contentRect.adjusted(-i, -i, i, i), 4, 4);
        int alpha = maxAlpha / (i + 1);
        painter.setPen(QPen(QColor(glowColor.red(), glowColor.green(), glowColor.blue(), alpha), 1.5));
        painter.setBrush(Qt::NoBrush);
        painter.drawPath(path);
    }
    
    // Draw the pixmap in the center
    painter.drawPixmap(contentRect, m_pixmap);
    
    // Draw a sharp outline directly around the image content
    QColor outlineColor = active ? QColor(52, 152, 219) : QColor(180, 180, 180);
    painter.setPen(QPen(outlineColor, 1.5));
    painter.drawRect(contentRect);
    
    // Draw text selection highlight if selecting or selection is active
    QRect selRect;
    if (isSelectingText) {
        selRect = QRect(selectStartPoint, selectEndPoint).normalized();
    } else if (m_hasSelection) {
        selRect = m_selectedRect;
    }
    
    if (!selRect.isNull()) {
        for (const auto &item : ocrItems) {
            if (selRect.intersects(item.rect)) {
                QRect drawHighlight = item.rect.intersected(contentRect);
                painter.fillRect(drawHighlight, QColor(52, 152, 219, 100));
            }
        }
    }
}

void PinWindow::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        QPoint pos = event->pos();
        
        bool clickedOnText = false;
        for (const auto &item : ocrItems) {
            if (item.rect.contains(pos)) {
                clickedOnText = true;
                break;
            }
        }
        
        if (clickedOnText) {
            isSelectingText = true;
            isDraggingWindow = false;
            selectStartPoint = pos;
            selectEndPoint = pos;
            m_hasSelection = false;
            m_selectedRect = QRect();
        } else {
            isSelectingText = false;
            isDraggingWindow = true;
            m_dragPosition = event->globalPos() - frameGeometry().topLeft();
            m_hasSelection = false;
            m_selectedRect = QRect();
            update();
        }
        event->accept();
    }
}

void PinWindow::mouseMoveEvent(QMouseEvent *event) {
    if (event->buttons() & Qt::LeftButton) {
        if (isSelectingText) {
            selectEndPoint = event->pos();
            update();
        } else if (isDraggingWindow) {
            move(event->globalPos() - m_dragPosition);
        }
        event->accept();
    }
}

void PinWindow::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        if (isSelectingText) {
            isSelectingText = false;
            
            QRect selRect = QRect(selectStartPoint, selectEndPoint).normalized();
            QList<OcrTextItem> selectedItems;
            for (const auto &item : ocrItems) {
                if (selRect.intersects(item.rect)) {
                    selectedItems.append(item);
                }
            }
            
            if (!selectedItems.isEmpty()) {
                m_hasSelection = true;
                m_selectedRect = selRect;
                
                std::sort(selectedItems.begin(), selectedItems.end(), [](const OcrTextItem &a, const OcrTextItem &b) {
                    if (qAbs(a.rect.top() - b.rect.top()) < 10) {
                        return a.rect.left() < b.rect.left();
                    }
                    return a.rect.top() < b.rect.top();
                });
                
                QStringList texts;
                for (const auto &item : selectedItems) {
                    texts.append(item.text);
                }
                
                QString copiedText = texts.join("\n");
                if (!copiedText.isEmpty()) {
                    QApplication::clipboard()->setText(copiedText);
                }
            }
            update();
        }
        isDraggingWindow = false;
        event->accept();
    }
}

void PinWindow::keyPressEvent(QKeyEvent *event) {
    if (event->key() == Qt::Key_Escape) {
        close();
        event->accept();
    } else {
        QWidget::keyPressEvent(event);
    }
}

void PinWindow::contextMenuEvent(QContextMenuEvent *event) {
    QMenu menu(this);
    menu.setStyleSheet(
        "QMenu {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 6px;"
        "  padding: 4px;"
        "}"
        "QMenu::item {"
        "  padding: 6px 20px;"
        "  border-radius: 4px;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "  font-size: 12px;"
        "  color: #2d3748;"
        "}"
        "QMenu::item:selected {"
        "  background-color: #f7fafc;"
        "  color: #3182ce;"
        "}"
    );
    QAction *copyAct = menu.addAction("⎘ 复制图片");
    QAction *closeAct = menu.addAction("✕ 关闭钉图");
    
    QAction *selected = menu.exec(event->globalPos());
    if (selected == copyAct) {
        QApplication::clipboard()->setPixmap(m_pixmap);
    } else if (selected == closeAct) {
        close();
    }
}

void PinWindow::enterEvent(QEnterEvent *event) {
    setFocus();
    activateWindow();
    raise();
    QWidget::enterEvent(event);
}

void PinWindow::changeEvent(QEvent *event) {
    if (event->type() == QEvent::ActivationChange) {
        update();
    }
    QWidget::changeEvent(event);
}
