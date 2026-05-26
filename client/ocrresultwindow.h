#pragma once
#include <QWidget>
#include <QPixmap>

class OcrResultWindow : public QWidget {
    Q_OBJECT
public:
    explicit OcrResultWindow(const QString &text, const QPixmap &pixmap, QWidget *parent = nullptr);
    ~OcrResultWindow() override;
protected:
    void keyPressEvent(QKeyEvent *event) override;
};
