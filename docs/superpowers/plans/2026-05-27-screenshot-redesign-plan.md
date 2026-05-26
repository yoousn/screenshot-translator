# Screenshot Translator Upgrade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the client's main screenshot toolbar with modern vector shape drawing annotations, precise animated snapping, outside-click copy, and a premium sidebar-based settings panel containing animated sliding toggle switches and auto-start on boot.

**Architecture:** Build clean, modular components inside the Qt C++ client. Subclass QAbstractButton to create an animated iOS-style toggle. Use QVariantAnimation for smooth snapping and sliding transitions. Use dynamic Win32 DWM APIs to resolve DPI-scaled window snapping bounds. Standardise QPixmap devicePixelRatio setting to ensure perfect coordinate alignment.

**Tech Stack:** C++17, Qt 5/6 Core/Gui/Widgets/Network, Win32 DWM (Desktop Window Manager) API, Registry QSettings.

---

### Task 1: Animated Toggle Switch Widget (`SwitchButton`)

**Files:**
- Create: `client/switchbutton.h`
- Create: `client/switchbutton.cpp`

- [ ] **Step 1: Create switchbutton.h**
  Create `client/switchbutton.h` containing the declaration for the animated iOS-style sliding switch button:
  ```cpp
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
  protected:
      void nextCheckState() override;
      void paintEvent(QPaintEvent *event) override;
      void setChecked(bool checked) override;
  private:
      qreal m_progress = 0.0;
      QVariantAnimation *anim = nullptr;
  };
  ```

- [ ] **Step 2: Create switchbutton.cpp**
  Create `client/switchbutton.cpp` containing the animation, track sliding, and track color fading logic:
  ```cpp
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
  ```

- [ ] **Step 3: Update client/CMakeLists.txt to build the new SwitchButton source files**
  Include `switchbutton.h` and `switchbutton.cpp` inside `CMakeLists.txt` targets.

- [ ] **Step 4: Verify Compilation**
  Run CMake configure and build in the build folder to ensure target builds cleanly.
  Command: `cmake --build build --config Release`
  Expected: Builds cleanly without warnings or errors.

---

### Task 2: Windows Precise Snapping with Transitions

**Files:**
- Modify: `client/screenshotwindow.h`
- Modify: `client/screenshotwindow.cpp`

- [ ] **Step 1: Declare animation parameters and properties in screenshotwindow.h**
  Add precise window tracking state and animation variables:
  ```cpp
  // Add this inside ScreenshotWindow private fields:
  QRectF currentHighlightRect;
  QRectF targetHighlightRect;
  QVariantAnimation *snapAnimation = nullptr;
  ```

- [ ] **Step 2: Update window scanning using DWM Extended Frame Bounds in screenshotwindow.cpp**
  In `EnumWindowsProc`, query `DWMWA_EXTENDED_FRAME_BOUNDS` for pixel-perfect coordinates, filtering background Cloaked Universal Apps:
  ```cpp
  #ifndef DWMWA_CLOAKED
  #define DWMWA_CLOAKED 14
  #endif
  #ifndef DWMWA_EXTENDED_FRAME_BOUNDS
  #define DWMWA_EXTENDED_FRAME_BOUNDS 9
  #endif

  BOOL CALLBACK EnumWindowsProc(HWND hwnd, LPARAM lParam) {
      EnumWindowsData *data = reinterpret_cast<EnumWindowsData*>(lParam);
      if (hwnd == data->currentHwnd) return TRUE;
      if (!IsWindowVisible(hwnd)) return TRUE;
      if (IsIconic(hwnd)) return TRUE;
      
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
          dwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &cloaked, sizeof(cloaked));
          if (cloaked) return TRUE;
      }
      
      LONG style = GetWindowLong(hwnd, GWL_STYLE);
      LONG exStyle = GetWindowLong(hwnd, GWL_EXSTYLE);
      if (exStyle & WS_EX_TOOLWINDOW) return TRUE;
      
      RECT r;
      bool gotRect = false;
      if (dwmGetWindowAttribute) {
          HRESULT hr = dwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &r, sizeof(r));
          if (SUCCEEDED(hr)) gotRect = true;
      }
      if (!gotRect) {
          if (!GetWindowRect(hwnd, &r)) return TRUE;
      }
      
      int w = r.right - r.left;
      int h = r.bottom - r.top;
      if (w > 40 && h > 40) {
          // Convert Win32 physical coordinates to logical coordinates using screen's devicePixelRatio
          qreal ratio = data->dpiRatio; // Ensure data structure carries this or query primaryScreen
          int lx = r.left / ratio;
          int ly = r.top / ratio;
          int lw = w / ratio;
          int lh = h / ratio;
          data->rects->append(QRect(lx, ly, lw, lh));
      }
      return TRUE;
  }
  ```

- [ ] **Step 3: Implement smooth QVariantAnimation transition for snapping rect in screenshotwindow.cpp**
  In the `ScreenshotWindow` constructor, initialize `snapAnimation`:
  ```cpp
  snapAnimation = new QVariantAnimation(this);
  snapAnimation->setDuration(180);
  snapAnimation->setEasingCurve(QEasingCurve::OutCubic);
  connect(snapAnimation, &QVariantAnimation::valueChanged, this, [this](const QVariant &val) {
      currentHighlightRect = val.toRectF();
      update();
  });
  ```
  In `mouseMoveEvent`, when the hovered snap rectangle changes, smoothly animate:
  ```cpp
  if (croppedRect.isEmpty()) {
      QRect snap = getSnappedRect(globalPos);
      QRectF target = QRectF(mapFromGlobal(snap.topLeft()), snap.size()).intersected(rect());
      if (target != targetHighlightRect) {
          targetHighlightRect = target;
          snapAnimation->stop();
          snapAnimation->setStartValue(currentHighlightRect.isEmpty() ? target : currentHighlightRect);
          snapAnimation->setEndValue(target);
          snapAnimation->start();
      }
  }
  ```

- [ ] **Step 4: Render the smooth snap frame in paintEvent**
  Modify `paintEvent` to draw the animated `currentHighlightRect` with anti-aliased dashes instead of `hoveredSnapRect`.

---

### Task 3: Redesigned Settings Panel

**Files:**
- Modify: `client/settingspanel.h`
- Modify: `client/settingspanel.cpp`

- [ ] **Step 1: Redesign settingspanel.h containing stacked layout and sidebar widgets**
  Update class declaration with a navigation `QListWidget` and page widgets:
  ```cpp
  #pragma once
  #include <QDialog>
  #include <QListWidget>
  #include <QStackedWidget>
  #include "switchbutton.h"
  // ... Include existing line edits and combos ...
  ```

- [ ] **Step 2: Implement page builder card layout in settingspanel.cpp**
  Inside constructor, set up sidebar list navigation on the left, and standard card pages on the right inside a `QStackedWidget`:
  - **Page 1: 服务与通道 (Services)**: N100 URL, authentication Token, translation channel dropdown.
  - **Page 2: 本地 OCR (Local OCR)**: Checkboxes replaced with sliding animated `SwitchButton` instances. Local engine directory input, browse button, and timeout.
  - **Page 3: 全局热键 (Hotkeys)**: Screenshot hotkey sequence editor.
  - **Page 4: 系统设置 (System)**: Auto-start on boot animated `SwitchButton` instance.
  - **Page 5: 关于 (About)**: Simple clean details card.

- [ ] **Step 3: Implement Autostart registry logic in settingspanel.cpp**
  Add Windows registry loader and saver for auto-start:
  ```cpp
  // Add auto-start registry operations in SettingsPanel::loadFields & saveFields using QSettings:
  void setRegistryAutostart(bool enable) {
      QSettings settings("HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run", QSettings::NativeFormat);
      QString appName = "ScreenshotTranslator";
      if (enable) {
          QString appPath = QDir::toNativeSeparators(QCoreApplication::applicationFilePath());
          settings.setValue(appName, "\"" + appPath + "\"");
      } else {
          settings.remove(appName);
      }
  }
  ```

- [ ] **Step 4: Style settings panel with modern CSS**
  Set stylesheet for card-based flat frames, sidebar highlight padding, and white backgrounds.

---

### Task 4: Main Screenshot Toolbar & Styling sub-panels

**Files:**
- Modify: `client/screenshotwindow.h`
- Modify: `client/screenshotwindow.cpp`

- [ ] **Step 1: Declare Main Toolbar and Styling Sub-toolbar layout in screenshotwindow.h**
  Add the sub-toolbar declaration and color styling trackers:
  ```cpp
  class FloatingToolbar : public QWidget {
      // ... Declare icon actions, sub-toolbar layout, and drawing style signals:
  signals:
      void drawToolSelected(int toolType);
      void colorSelected(const QColor &color);
      void widthSelected(int width);
  };
  ```

- [ ] **Step 2: Create elegant QPainter-based vector buttons in screenshotwindow.cpp**
  Implement vector rendering inside toolbar button painting to keep it asset-free and crisp under High-DPI screens.
  - Custom pill layout with grip handles.
  - Selection tools: Rect, Circle, Arrow, Pen.
  - Actions: Undo, OCR, Translate, Pin, Save, Settings, Close, Copy.

- [ ] **Step 3: Build the sub-row style panel below the main toolbar**
  Implement a dynamic secondary capsule pill holding 5 colored dots (Red, Blue, Green, Yellow, White) and 3 brush thickness selectors.
  Configure visibility to show and slide out below the main toolbar when any drawing annotation tool is active.

---

### Task 5: Complete Annotation Layer & Undo/Redo Engine

**Files:**
- Modify: `client/screenshotwindow.h`
- Modify: `client/screenshotwindow.cpp`

- [ ] **Step 1: Declare Annotation structures and mouse action trackers in screenshotwindow.h**
  Add shape data definitions and state machine fields:
  ```cpp
  enum AnnotationType { AnnotateNone, AnnotateRect, AnnotateCircle, AnnotateArrow, AnnotatePen };
  struct Annotation {
      AnnotationType type;
      QRect rect;
      QList<QPoint> points;
      QColor color = QColor(229, 62, 62); // Default Premium Red
      int width = 3;
  };
  // Inside ScreenshotWindow:
  AnnotationType activeTool = AnnotateNone;
  QColor activeColor = QColor(229, 62, 62);
  int activeWidth = 3;
  QList<Annotation> annotations;
  QList<Annotation> undoHistory;
  Annotation currentDraft;
  bool isDrawingAnnotation = false;
  ```

- [ ] **Step 2: Implement annotation painting inside paintEvent**
  Draw all complete annotations plus the current drafting preview. Use dynamic anti-aliased math for drawing arrows:
  ```cpp
  void drawArrow(QPainter &painter, const QPoint &start, const QPoint &end, const QColor &color, int width) {
      painter.save();
      painter.setPen(QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
      painter.drawLine(start, end);
      
      double angle = std::atan2(end.y() - start.y(), end.x() - start.x());
      qreal headLength = 15 + width;
      QPointF p1 = end - QPointF(std::cos(angle + M_PI/6) * headLength, std::sin(angle + M_PI/6) * headLength);
      QPointF p2 = end - QPointF(std::cos(angle - M_PI/6) * headLength, std::sin(angle - M_PI/6) * headLength);
      
      painter.setBrush(color);
      QPolygonF head;
      head << end << p1 << p2;
      painter.drawPolygon(head);
      painter.restore();
  }
  ```

- [ ] **Step 3: Redirect Mouse Events when drawing is active**
  In `mousePressEvent`, `mouseMoveEvent`, and `mouseReleaseEvent`, if `activeTool != AnnotateNone`, capture mouse movements to update `currentDraft` drawing paths inside `croppedRect` instead of resizing or moving the screenshot box.

- [ ] **Step 4: Implement Undo/Redo operations**
  Hook up toolbar slots to pop/push items on the `annotations` and `undoHistory` lists, triggering window updates.

---

### Task 6: Outside-Click Quick Copy & High-DPI Alignment Polish

**Files:**
- Modify: `client/screenshotwindow.cpp`

- [ ] **Step 1: Standardise QPixmap Device Pixel Ratio on received image bytes**
  Upon translation network call completion, set loaded QPixmap ratio:
  ```cpp
  // Inside triggerTranslation:
  currentImage = resPixmap;
  currentImage.setDevicePixelRatio(devicePixelRatio());
  ```

- [ ] **Step 2: Scale local OCR selection rectangles inside PinWindow**
  Inside `PinWindow::applyOcrResults`, divide bounding box coordinates by the screen's DPI scale factor to allow text selection bounding borders to line up perfectly:
  ```cpp
  qreal ratio = devicePixelRatio();
  int x1 = p1.at(0).toDouble() / ratio;
  int y1 = p1.at(1).toDouble() / ratio;
  int x3 = p3.at(0).toDouble() / ratio;
  int y3 = p3.at(1).toDouble() / ratio;
  item.rect = QRect(QPoint(x1, y1), QPoint(x3, y3)).translated(10, 10);
  ```

- [ ] **Step 3: Handle Outside Selection Clicks**
  In `ScreenshotWindow::mousePressEvent`, if `!croppedRect.isEmpty()` and `!croppedRect.contains(event->pos())` and `getHandleAt(event->pos()) < 0`, quickly call `copyToClipboard()` and close.

- [ ] **Step 4: Final verification and build check**
  Compile in release mode and verify flawless snapping, drawing annotations, quick copying, and Settings layout scaling.
