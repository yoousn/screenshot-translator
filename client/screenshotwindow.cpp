#include "screenshotwindow.h"
#include "settingspanel.h"
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

// ----------------------------------------------------
// 1. FloatingToolbar Implementation
// ----------------------------------------------------
FloatingToolbar::FloatingToolbar(QWidget *parent) : QWidget(parent) {
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_TranslucentBackground);
    
    QHBoxLayout *layout = new QHBoxLayout(this);
    layout->setContentsMargins(10, 10, 10, 10);
    layout->setSpacing(0);
    
    // 主容器，应用极简现代白色拟态与边框样式
    QWidget *container = new QWidget(this);
    container->setStyleSheet(
        "QWidget {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 8px;"
        "}"
    );
    QHBoxLayout *containerLayout = new QHBoxLayout(container);
    containerLayout->setContentsMargins(6, 4, 6, 4);
    containerLayout->setSpacing(4);
    
    // 增加精致的投影效果
    QGraphicsDropShadowEffect *shadow = new QGraphicsDropShadowEffect(this);
    shadow->setBlurRadius(10);
    shadow->setColor(QColor(0, 0, 0, 30));
    shadow->setOffset(0, 3);
    container->setGraphicsEffect(shadow);
    
    QString btnStyle = 
        "QPushButton {"
        "  color: #4a5568;"
        "  background-color: transparent;"
        "  border: none;"
        "  padding: 6px 10px;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "  font-size: 12px;"
        "  font-weight: 500;"
        "  border-radius: 6px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #f7fafc;"
        "  color: #1890ff;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #edf2f7;"
        "  color: #096dd9;"
        "}";
        
    transBtn = new QPushButton("文 翻译", container);
    transBtn->setToolTip("翻译选区并嵌字 (Ctrl+Q)");
    transBtn->setStyleSheet(btnStyle);
    transBtn->setEnabled(false); // 默认禁用，等OCR检测到文字后再启用
    connect(transBtn, &QPushButton::clicked, this, &FloatingToolbar::translateRequested);
    containerLayout->addWidget(transBtn);
    // 初始为灰色禁用状态
    setTranslateEnabled(false);
    
    QPushButton *pinBtn = new QPushButton("📌 钉图", container);
    pinBtn->setToolTip("将图片钉在桌面上");
    pinBtn->setStyleSheet(btnStyle);
    connect(pinBtn, &QPushButton::clicked, this, &FloatingToolbar::pinRequested);
    containerLayout->addWidget(pinBtn);
    
    QPushButton *saveBtn = new QPushButton("💾 保存", container);
    saveBtn->setToolTip("保存图片到本地 (Ctrl+S)");
    saveBtn->setStyleSheet(btnStyle);
    connect(saveBtn, &QPushButton::clicked, this, &FloatingToolbar::saveRequested);
    containerLayout->addWidget(saveBtn);
    
    QPushButton *settingsBtn = new QPushButton("⚙ 设置", container);
    settingsBtn->setToolTip("打开配置面板");
    settingsBtn->setStyleSheet(btnStyle);
    connect(settingsBtn, &QPushButton::clicked, this, &FloatingToolbar::settingsRequested);
    containerLayout->addWidget(settingsBtn);
    
    // 添加分割线
    QFrame *separator = new QFrame(container);
    separator->setFrameShape(QFrame::VLine);
    separator->setFrameShadow(QFrame::Sunken);
    separator->setStyleSheet("color: #e2e8f0; max-height: 16px; margin: 0 4px;");
    containerLayout->addWidget(separator);
    
    QPushButton *closeBtn = new QPushButton("✕ 取消", container);
    closeBtn->setToolTip("关闭截图 (Esc)");
    closeBtn->setStyleSheet(
        "QPushButton {"
        "  color: #e53e3e;"
        "  background-color: transparent;"
        "  border: none;"
        "  padding: 6px 10px;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "  font-size: 12px;"
        "  font-weight: 500;"
        "  border-radius: 6px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #fff5f5;"
        "  color: #c53030;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #fed7d7;"
        "  color: #9b2c2c;"
        "}"
    );
    connect(closeBtn, &QPushButton::clicked, this, &FloatingToolbar::closeRequested);
    containerLayout->addWidget(closeBtn);
    
    QPushButton *copyBtn = new QPushButton("⎘ 复制", container);
    copyBtn->setToolTip("复制当前图片到剪贴板 (Ctrl+C)");
    copyBtn->setStyleSheet(btnStyle);
    connect(copyBtn, &QPushButton::clicked, this, &FloatingToolbar::copyRequested);
    containerLayout->addWidget(copyBtn);
    
    layout->addWidget(container);
    adjustSize();
}

void FloatingToolbar::setTranslateEnabled(bool enabled) {
    if (!transBtn) return;
    transBtn->setEnabled(enabled);
    if (enabled) {
        transBtn->setStyleSheet(
            "QPushButton {"
            "  color: #4a5568;"
            "  background-color: transparent;"
            "  border: none;"
            "  padding: 6px 10px;"
            "  font-family: 'Segoe UI', 'Microsoft YaHei';"
            "  font-size: 12px;"
            "  font-weight: 500;"
            "  border-radius: 6px;"
            "}"
            "QPushButton:hover {"
            "  background-color: #f7fafc;"
            "  color: #1890ff;"
            "}"
            "QPushButton:pressed {"
            "  background-color: #edf2f7;"
            "  color: #096dd9;"
            "}"
        );
    } else {
        transBtn->setStyleSheet(
            "QPushButton {"
            "  color: #cbd5e0;"
            "  background-color: transparent;"
            "  border: none;"
            "  padding: 6px 10px;"
            "  font-family: 'Segoe UI', 'Microsoft YaHei';"
            "  font-size: 12px;"
            "  font-weight: 500;"
            "  border-radius: 6px;"
            "}"
        );
    }
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
};

BOOL CALLBACK EnumWindowsProc(HWND hwnd, LPARAM lParam) {
    EnumWindowsData *data = reinterpret_cast<EnumWindowsData*>(lParam);
    if (hwnd == data->currentHwnd) return TRUE;
    if (!IsWindowVisible(hwnd)) return TRUE;
    if (IsIconic(hwnd)) return TRUE;
    
    LONG style = GetWindowLong(hwnd, GWL_STYLE);
    LONG exStyle = GetWindowLong(hwnd, GWL_EXSTYLE);
    if (exStyle & WS_EX_TOOLWINDOW) return TRUE;
    
    RECT r;
    if (GetWindowRect(hwnd, &r)) {
        int w = r.right - r.left;
        int h = r.bottom - r.top;
        if (w > 40 && h > 40) {
            data->rects->append(QRect(r.left, r.top, w, h));
        }
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
    
    // Support ESC to close the screenshot window bulletproofly
    QShortcut *escShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    connect(escShortcut, &QShortcut::activated, this, &QWidget::close);
    
    // 抓取全屏
    QScreen *screen = QApplication::primaryScreen();
    if (screen) {
        fullScreenPixmap = screen->grabWindow(0);
    }
    
    fullScreenImage = fullScreenPixmap.toImage();
    
    // 初始化旋转加载定时器
    spinnerTimer = new QTimer(this);
    connect(spinnerTimer, &QTimer::timeout, this, [=]() {
        spinnerAngle = (spinnerAngle + 12) % 360;
        if (isLoading) {
            update();
        }
    });
    spinnerTimer->start(50);
    
    // 窗口探测 (Win32 API)
#ifdef Q_OS_WIN
    EnumWindowsData data;
    data.rects = &detectedWindowRects;
    data.currentHwnd = (HWND)this->winId();
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
    if (croppedRect.isEmpty() && !hoveredSnapRect.isEmpty()) {
        painter.drawPixmap(hoveredSnapRect, fullScreenPixmap, hoveredSnapRect);
        painter.setPen(QPen(QColor(24, 144, 255), 2, Qt::DashLine));
        painter.drawRect(hoveredSnapRect);
    }
    
    if (isDragging || !croppedRect.isEmpty()) {
        // 重新照亮选区
        painter.drawPixmap(croppedRect, fullScreenPixmap, croppedRect);
        
        // 如果已经返回了翻译结果图，将其绘制覆盖于选区上
        if (isTranslated && !currentImage.isNull()) {
            painter.drawPixmap(croppedRect, currentImage);
        }
        
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
            isTranslated = false;
            hideToolbar();
            update();
        } else {
            close();
        }
        return;
    }

    if (event->button() == Qt::LeftButton) {
        pressPos = event->pos();
        int handle = getHandleAt(event->pos());
        
        if (handle >= 0) {
            captureState = StateResizing;
            activeHandle = handle;
            isDragging = true;
            hideToolbar();
        } else if (!croppedRect.isEmpty() && croppedRect.contains(event->pos())) {
            captureState = StateMoving;
            dragOffset = event->pos() - croppedRect.topLeft();
            isDragging = true;
            hideToolbar();
        } else {
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
            hoveredSnapRect = QRect(mapFromGlobal(snap.topLeft()), snap.size());
            hoveredSnapRect = hoveredSnapRect.intersected(rect());
            update();
        }
    }
}

void ScreenshotWindow::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && isDragging) {
        isDragging = false;
        
        if (captureState == StateSelecting) {
            int dist = (event->pos() - pressPos).manhattanLength();
            if (dist < 6) {
                if (!hoveredSnapRect.isEmpty() && hoveredSnapRect.width() > 10 && hoveredSnapRect.height() > 10) {
                    croppedRect = hoveredSnapRect;
                } else {
                    croppedRect = QRect();
                }
            }
        }
        
        captureState = StateIdle;
        
        if (croppedRect.width() > 10 && croppedRect.height() > 10) {
            currentImage = fullScreenPixmap.copy(croppedRect);
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

void ScreenshotWindow::showToolbar() {
    if (!toolbar) {
        toolbar = new FloatingToolbar();
        connect(toolbar, &FloatingToolbar::translateRequested, this, &ScreenshotWindow::triggerTranslation);
        connect(toolbar, &FloatingToolbar::copyRequested, this, &ScreenshotWindow::copyToClipboard);
        connect(toolbar, &FloatingToolbar::saveRequested, this, &ScreenshotWindow::saveToFile);
        connect(toolbar, &FloatingToolbar::pinRequested, this, &ScreenshotWindow::pinImage);
        connect(toolbar, &FloatingToolbar::settingsRequested, this, [=]() {
            SettingsPanel panel;
            if (panel.exec() == QDialog::Accepted) {
                config.load();
            }
        });
        connect(toolbar, &FloatingToolbar::closeRequested, this, &ScreenshotWindow::close);
    }
    // 每次显示工具栏时，翻译按钮默认禁用灰色，异步OCR检测后再启用
    hasDetectedText = false;
    toolbar->setTranslateEnabled(false);
    updateToolbarPosition();
    toolbar->show();
    // 异步检测选区是否包含可翻译文字
    checkForText();
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

void ScreenshotWindow::checkForText() {
    if (croppedRect.isEmpty()) return;
    QPixmap crop = fullScreenPixmap.copy(croppedRect);
    netClient->ocrImage(crop, config, [this](bool success, const QJsonArray &ocrResults) {
        if (success) {
            if (ocrResults.size() > 0) {
                hasDetectedText = true;
                if (toolbar) toolbar->setTranslateEnabled(true);
            } else {
                hasDetectedText = false;
                if (toolbar) toolbar->setTranslateEnabled(false);
            }
        } else {
            // OCR 接口调用失败（比如 404 或 500 等），降级处理：默认允许翻译，不作限制
            hasDetectedText = true;
            if (toolbar) toolbar->setTranslateEnabled(true);
        }
    });
}

void ScreenshotWindow::triggerTranslation() {
    if (isLoading || croppedRect.isEmpty() || !hasDetectedText) return;
    
    isLoading = true;
    if (toolbar) toolbar->setTranslateEnabled(false);
    update();
    
    QPixmap originalCrop = fullScreenPixmap.copy(croppedRect);
    
    netClient->translateImage(originalCrop, config, [=](bool success, const QPixmap &resPixmap) {
        isLoading = false;
        if (success && !resPixmap.isNull()) {
            currentImage = resPixmap;
            isTranslated = true;
        }
        if (toolbar) toolbar->setTranslateEnabled(hasDetectedText);
        update();
    });
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
    
    // Support ESC to close the pinned window bulletproofly
    QShortcut *escShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    connect(escShortcut, &QShortcut::activated, this, &QWidget::close);
    
    // Asynchronously request OCR on the pinned image in the background
    queryOcr();
}

PinWindow::~PinWindow() {
}

void PinWindow::queryOcr() {
    netClient->ocrImage(m_pixmap, config, [this](bool success, const QJsonArray &ocrResults) {
        if (success) {
            ocrItems.clear();
            for (const auto &val : ocrResults) {
                QJsonObject obj = val.toObject();
                QJsonArray box = obj.value("box").toArray();
                if (box.size() >= 4) {
                    QJsonArray p1 = box.at(0).toArray();
                    QJsonArray p3 = box.at(2).toArray();
                    int x1 = p1.at(0).toDouble();
                    int y1 = p1.at(1).toDouble();
                    int x3 = p3.at(0).toDouble();
                    int y3 = p3.at(1).toDouble();
                    
                    OcrTextItem item;
                    // Translate coordinates by +10 because the image is painted at (10, 10)
                    item.rect = QRect(QPoint(x1, y1), QPoint(x3, y3)).translated(10, 10);
                    item.text = obj.value("text").toString();
                    ocrItems.append(item);
                }
            }
        }
    });
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
