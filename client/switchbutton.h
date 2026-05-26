#pragma once
#include <QAbstractButton>
#include <QVariantAnimation>

class SwitchButton : public QAbstractButton {
    Q_OBJECT
    Q_PROPERTY(qreal progress READ progress WRITE setProgress)
public:
    explicit SwitchButton(QWidget *parent = nullptr);
    QSize sizeHint() const override;
    qreal progress() const { return m_progress; }
    void setProgress(qreal p);
    void setChecked(bool checked);
protected:
    void nextCheckState() override;
    void paintEvent(QPaintEvent *event) override;
private:
    qreal m_progress = 0.0;
    QVariantAnimation *anim = nullptr;
};
