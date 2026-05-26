#include "screenshotwindow.h"
#include "settingspanel.h"
#include <QApplication>
#include <QScreen>
#include <QPainter>
#include <QMouseEvent>
#include <QClipboard>
#include <QFileDialog>
#include <QMenu>
#include <QKeySequence>

// ----------------------------------------------------
// 1. FloatingToolbar Implementation
// ----------------------------------------------------
FloatingToolbar::FloatingToolbar(QWidget *parent) : QWidget(parent) {
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_TranslucentBackground);
    
    QHBoxLayout *layout = new QHBoxLayout(this);
    layout->setContentsMargins(5, 2, 5, 2);
    layout->setSpacing(4);
    
    // 主容器，应用精致的 HSL 渐变和阴影样式（现代玻璃拟态）
    QWidget *container = new QWidget(this);
    container->setStyleSheet(
        "QWidget {"
        "  background-color: qlineargradient(x1:0, y1:0, x2:0, y2:1, stop:0 #2c3e50, stop:1 #1a252f);"
        "  border: 1px solid #34495e;"
        "  border-radius: 6px;"
        "}"
    );
    QHBoxLayout *containerLayout = new QHBoxLayout(container);
    containerLayout->setContentsMargins(6, 4, 6, 4);
    containerLayout->setSpacing(6);
    
    QString btnStyle = 
        "QPushButton {"
        "  color: #ecf0f1;"
        "  background-color: transparent;"
        "  border: none;"
        "  padding: 4px 8px;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "  font-size: 12px;"
        "  border-radius: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #3498db;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #2980b9;"
        "}";
        
    QPushButton *transBtn = new QPushButton("译 翻译", container);
    transBtn->setToolTip("翻译选区并嵌字 (Ctrl+Q)");
    transBtn->setStyleSheet(
        "QPushButton {"
        "  color: #ffffff;"
        "  background-color: #e74c3c;"
        "  border: none;"
        "  padding: 4px 8px;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "  font-size: 12px;"
        "  font-weight: bold;"
        "  border-radius: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #c0392b;"
        "}"
    );
    connect(transBtn, &QPushButton::clicked, this, &FloatingToolbar::translateRequested);
    containerLayout->addWidget(transBtn);
    
    QPushButton *copyBtn = new QPushButton("复制", container);
    copyBtn->setToolTip("复制当前图片到剪贴板 (Ctrl+C)");
    copyBtn->setStyleSheet(btnStyle);
    connect(copyBtn, &QPushButton::clicked, this, &FloatingToolbar::copyRequested);
    containerLayout->addWidget(copyBtn);
    
    QPushButton *saveBtn = new QPushButton("保存", container);
    saveBtn->setToolTip("保存图片到本地 (Ctrl+S)");
    saveBtn->setStyleSheet(btnStyle);
    connect(saveBtn, &QPushButton::clicked, this, &FloatingToolbar::saveRequested);
    containerLayout->addWidget(saveBtn);
    
    QPushButton *pinBtn = new QPushButton("钉图", container);
    pinBtn->setToolTip("将图片钉在桌面上");
    pinBtn->setStyleSheet(btnStyle);
    connect(pinBtn, &QPushButton::clicked, this, &FloatingToolbar::pinRequested);
    containerLayout->addWidget(pinBtn);
    
    QPushButton *settingsBtn = new QPushButton("设置", container);
    settingsBtn->setToolTip("打开配置面板");
    settingsBtn->setStyleSheet(btnStyle);
    connect(settingsBtn, &QPushButton::clicked, this, &FloatingToolbar::settingsRequested);
    containerLayout->addWidget(settingsBtn);
    
    QPushButton *closeBtn = new QPushButton("取消", container);
    closeBtn->setToolTip("关闭截图 (Esc)");
    closeBtn->setStyleSheet(
        "QPushButton {"
        "  color: #bdc3c7;"
        "  background-color: transparent;"
        "  border: none;"
        "  padding: 4px 8px;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "  font-size: 12px;"
        "  border-radius: 4px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #7f8c8d;"
        "  color: white;"
        "}"
    );
    connect(closeBtn, &QPushButton::clicked, this, &FloatingToolbar::closeRequested);
    containerLayout->addWidget(closeBtn);
    
    layout->addWidget(container);
    adjustSize();
}

// ----------------------------------------------------
// 2. ScreenshotWindow Implementation
// ----------------------------------------------------
ScreenshotWindow::ScreenshotWindow(QWidget *parent) : QWidget(parent) {
    setWindowFlags(Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool);
    setAttribute(Qt::WA_DeleteOnClose);
    setFocusPolicy(Qt::StrongFocus);
    
    config.load();
    netClient = new NetworkClient(this);
    
    // 抓取全屏
    QScreen *screen = QApplication::primaryScreen();
    if (screen) {
        fullScreenPixmap = screen->grabWindow(0);
    }
    
    showFullScreen();
    setCursor(Qt::CrossCursor);
}

ScreenshotWindow::~ScreenshotWindow() {
    hideToolbar();
}

void ScreenshotWindow::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.drawPixmap(0, 0, fullScreenPixmap);
    
    // 半透明灰色遮罩
    painter.fillRect(rect(), QColor(0, 0, 0, 110));
    
    if (isDragging || !croppedRect.isEmpty()) {
        // 重新照亮选区
        painter.drawPixmap(croppedRect, fullScreenPixmap, croppedRect);
        
        // 如果已经返回了翻译结果图，将其绘制覆盖于选区上
        if (isTranslated && !currentImage.isNull()) {
            painter.drawPixmap(croppedRect, currentImage);
        }
        
        // 绘制选区亮绿色边框
        painter.setPen(QPen(QColor(46, 204, 113), 2));
        painter.drawRect(croppedRect);
        
        // 绘制选区的大小指示器 (如: 400 x 300)
        QString sizeText = QString("%1 x %2").arg(croppedRect.width()).arg(croppedRect.height());
        painter.setPen(Qt::white);
        painter.setFont(QFont("Segoe UI", 9));
        painter.drawText(croppedRect.left() + 4, croppedRect.top() - 6, sizeText);
        
        // 正在翻译的加载中状态
        if (isLoading) {
            painter.fillRect(croppedRect, QColor(0, 0, 0, 160));
            painter.setPen(Qt::white);
            QFont f("Segoe UI", 12);
            f.setBold(true);
            painter.setFont(f);
            painter.drawText(croppedRect, Qt::AlignCenter, "正在智能翻译与无痕嵌字中...");
        }
    }
}

void ScreenshotWindow::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        // 如果工具栏已显示且点击在工具栏外，重新拖动
        if (toolbar && toolbar->isVisible()) {
            hideToolbar();
            isTranslated = false;
        }
        
        startPoint = event->pos();
        endPoint = startPoint;
        croppedRect = QRect();
        isDragging = true;
        update();
    }
}

void ScreenshotWindow::mouseMoveEvent(QMouseEvent *event) {
    if (isDragging) {
        endPoint = event->pos();
        croppedRect = QRect(startPoint, endPoint).normalized();
        update();
    }
}

void ScreenshotWindow::mouseReleaseEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton && isDragging) {
        isDragging = false;
        croppedRect = QRect(startPoint, endPoint).normalized();
        
        if (croppedRect.width() > 10 && croppedRect.height() > 10) {
            currentImage = fullScreenPixmap.copy(croppedRect);
            showToolbar();
        } else {
            croppedRect = QRect();
            update();
        }
    }
}

void ScreenshotWindow::keyPressEvent(QKeyEvent *event) {
    // 快捷键拦截
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
                config.load(); // 刷新配置
            }
        });
        connect(toolbar, &FloatingToolbar::closeRequested, this, &ScreenshotWindow::close);
    }
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
    if (isLoading || croppedRect.isEmpty()) return;
    
    isLoading = true;
    update();
    
    QPixmap originalCrop = fullScreenPixmap.copy(croppedRect);
    
    netClient->translateImage(originalCrop, config, [=](bool success, const QPixmap &resPixmap) {
        isLoading = false;
        if (success && !resPixmap.isNull()) {
            currentImage = resPixmap;
            isTranslated = true;
        }
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
    
    resize(pixmap.size());
    move(pos);
}

void PinWindow::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    // 绘制阴影和发光边缘，极具高级感
    painter.setRenderHint(QPainter::Antialiasing);
    painter.fillRect(rect(), Qt::transparent);
    painter.drawPixmap(rect(), m_pixmap);
    
    // 蓝色窄边框
    painter.setPen(QPen(QColor(52, 152, 219), 2));
    painter.drawRect(rect().adjusted(1, 1, -1, -1));
}

void PinWindow::mousePressEvent(QMouseEvent *event) {
    if (event->button() == Qt::LeftButton) {
        m_dragPosition = event->globalPos() - frameGeometry().topLeft();
        event->accept();
    }
}

void PinWindow::mouseMoveEvent(QMouseEvent *event) {
    if (event->buttons() & Qt::LeftButton) {
        move(event->globalPos() - m_dragPosition);
        event->accept();
    }
}

void PinWindow::contextMenuEvent(QContextMenuEvent *event) {
    QMenu menu(this);
    QAction *copyAct = menu.addAction("复制图片");
    QAction *closeAct = menu.addAction("关闭钉图");
    
    QAction *selected = menu.exec(event->globalPos());
    if (selected == copyAct) {
        QApplication::clipboard()->setPixmap(m_pixmap);
    } else if (selected == closeAct) {
        close();
    }
}
