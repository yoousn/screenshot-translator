#include "switchbutton.h"
#include <QPainter>

SwitchButton::SwitchButton(QWidget *parent) : QAbstractButton(parent) {
    setCheckable(true);
    setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Fixed);
    anim = new QVariantAnimation(this);
    anim->setDuration(180);
    anim->setEasingCurve(QEasingCurve::InOutQuad);
    connect(anim, &QVariantAnimation::valueChanged, this, [this](const QVariant &value) {
        m_progress = value.toReal();
        update();
    });
}

QSize SwitchButton::sizeHint() const {
    return QSize(44, 20);
}

void SwitchButton::setProgress(qreal p) {
    m_progress = p;
    update();
}

void SwitchButton::nextCheckState() {
    setChecked(!isChecked());
    anim->stop();
    anim->setStartValue(m_progress);
    anim->setEndValue(isChecked() ? 1.0 : 0.0);
    anim->start();
    emit toggled(isChecked());
}

void SwitchButton::setChecked(bool checked) {
    QAbstractButton::setChecked(checked);
    m_progress = checked ? 1.0 : 0.0;
    update();
}

void SwitchButton::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    QColor unselectedColor(203, 213, 224); // #cbd5e0
    QColor selectedColor(24, 144, 255);    // #1890ff
    QColor bgColor = QColor::fromRgbF(
        unselectedColor.redF()   + m_progress * (selectedColor.redF()   - unselectedColor.redF()),
        unselectedColor.greenF() + m_progress * (selectedColor.greenF() - unselectedColor.greenF()),
        unselectedColor.blueF()  + m_progress * (selectedColor.blueF()  - unselectedColor.blueF())
    );

    painter.setPen(Qt::NoPen);
    painter.setBrush(bgColor);
    painter.drawRoundedRect(rect(), height() / 2.0, height() / 2.0);

    qreal margin = 2.0;
    qreal thumbSize = height() - 2.0 * margin;
    qreal startX = margin;
    qreal endX = width() - thumbSize - margin;
    qreal x = startX + m_progress * (endX - startX);

    QRectF thumbRect(x, margin, thumbSize, thumbSize);
    painter.setBrush(QColor(0, 0, 0, 25));
    painter.drawEllipse(thumbRect.translated(0, 1));
    painter.setBrush(Qt::white);
    painter.drawEllipse(thumbRect);
}
