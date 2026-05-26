#include "ocrresultwindow.h"
#include <QApplication>
#include <QClipboard>
#include <QTimer>
#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QSplitter>
#include <QTextEdit>
#include <QScrollArea>
#include <QLabel>
#include <QPushButton>

OcrResultWindow::OcrResultWindow(const QString &text, const QPixmap &pixmap, QWidget *parent)
    : QWidget(parent) {
    setWindowTitle("文字识别结果");
    resize(500, 650);
    setWindowFlags(Qt::Window | Qt::WindowMinMaxButtonsHint | Qt::WindowCloseButtonHint);
    
    // Set modern white stylesheet matching the system theme
    setStyleSheet(
        "QWidget {"
        "  background-color: #f7fafc;"
        "  font-family: 'Segoe UI', 'Microsoft YaHei';"
        "}"
        "QTextEdit {"
        "  background-color: #ffffff;"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 6px;"
        "  padding: 8px;"
        "  color: #2d3748;"
        "  font-size: 14px;"
        "  line-height: 1.4;"
        "}"
        "QScrollArea {"
        "  border: 1px solid #e2e8f0;"
        "  border-radius: 6px;"
        "  background-color: #edf2f7;"
        "}"
        "QPushButton {"
        "  color: #ffffff;"
        "  background-color: #1890ff;"
        "  border: none;"
        "  border-radius: 6px;"
        "  padding: 8px 16px;"
        "  font-weight: bold;"
        "  font-size: 13px;"
        "}"
        "QPushButton:hover {"
        "  background-color: #40a9ff;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #096dd9;"
        "}"
    );

    QVBoxLayout *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(15, 15, 15, 15);
    mainLayout->setSpacing(10);

    QSplitter *splitter = new QSplitter(Qt::Vertical, this);
    splitter->setStyleSheet("QSplitter::handle { background-color: #cbd5e0; height: 4px; }");

    // Text box (Top)
    QTextEdit *textEdit = new QTextEdit(this);
    textEdit->setPlainText(text);
    textEdit->setPlaceholderText("未识别到文字");
    splitter->addWidget(textEdit);

    // Image container (Bottom)
    QScrollArea *scrollArea = new QScrollArea(this);
    scrollArea->setWidgetResizable(true);
    
    QLabel *imageLabel = new QLabel(scrollArea);
    imageLabel->setPixmap(pixmap);
    imageLabel->setAlignment(Qt::AlignCenter);
    imageLabel->setStyleSheet("background-color: #edf2f7; border: none;");
    
    scrollArea->setWidget(imageLabel);
    splitter->addWidget(scrollArea);

    // Give text edit more initial space than image
    splitter->setStretchFactor(0, 3);
    splitter->setStretchFactor(1, 2);

    mainLayout->addWidget(splitter);

    // Buttons
    QHBoxLayout *btnLayout = new QHBoxLayout();
    btnLayout->setSpacing(10);
    
    QPushButton *copyBtn = new QPushButton("⎘ 复制全部文本", this);
    connect(copyBtn, &QPushButton::clicked, [=]() {
        QApplication::clipboard()->setText(textEdit->toPlainText());
        // Quick feedback
        copyBtn->setText("✓ 已复制！");
        QTimer::singleShot(1500, [=]() {
            copyBtn->setText("⎘ 复制全部文本");
        });
    });
    
    QPushButton *closeBtn = new QPushButton("关闭", this);
    closeBtn->setStyleSheet(
        "QPushButton {"
        "  color: #4a5568;"
        "  background-color: #edf2f7;"
        "  border: 1px solid #cbd5e0;"
        "}"
        "QPushButton:hover {"
        "  background-color: #e2e8f0;"
        "}"
        "QPushButton:pressed {"
        "  background-color: #cbd5e0;"
        "}"
    );
    connect(closeBtn, &QPushButton::clicked, this, &QWidget::close);

    btnLayout->addWidget(copyBtn);
    btnLayout->addStretch();
    btnLayout->addWidget(closeBtn);

    mainLayout->addLayout(btnLayout);
}

OcrResultWindow::~OcrResultWindow() {
}
