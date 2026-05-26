# Screenshot Translator Annotation and Settings Redesign Spec

This specification document outlines the technical design for upgrading the Screenshot Translator client. The redesign includes a modern, icon-only screenshot toolbar, precise window snapping with fluid transitions, an rich annotation layer (rectangle, circle, arrow, pen), outside-click quick copy, a premium sidebar-based settings panel, an auto-start on boot system, and animated toggle switches.

---

## 1. Architectural & UIRedesign

### 1.1 Custom Main Toolbar & Styling
We will redesign the `FloatingToolbar` as a highly aesthetic horizontal pill.
- **Visuals**: Frameless, transparent background window using a capsule container:
  - Width fits icons, height 40px, border-radius 20px.
  - Border: `1px solid #e2e8f0`, background-color: `#ffffff`.
  - Drop shadow: subtle blur 15px, `#000000` with 8% opacity.
- **Button Icons**: Custom painted vector elements or crisp SVG icons styled with clean CSS:
  - Select/Move tool, Rectangle, Circle, Arrow, Pen, Undo, OCR, Translate, Pin, Save, Settings, Close, Copy.
  - Interactive states: smooth transition on hover (`background-color: #f1f5f9; color: #3b82f6;`) and press.
- **Style Sub-Toolbar**: A secondary pill appears 6px directly below the main toolbar when a drawing tool is active:
  - Height 34px, rounded capsule.
  - Colors: Red (`#e53e3e`), Blue (`#3b82f6`), Green (`#10b981`), Yellow (`#f59e0b`), White (`#ffffff`).
  - Line thicknesses: Thin (2px), Medium (4px), Thick (6px).

### 1.2 Precise Window Snapping & Transition Animations
- **DWM Frame Bounds**: Update `EnumWindowsProc` to use `DwmGetWindowAttribute` with `DWMWA_EXTENDED_FRAME_BOUNDS` to retrieve precise visible rectangles and bypass invisible shadow margins.
- **Cloaked Filtering**: Check `DWMWA_CLOAKED` to automatically filter out background UWP apps.
- **Smooth Animation**:
  - Add `QVariantAnimation` to interpolate `currentHighlightRect` (represented as `QRectF`) smoothly towards `targetHighlightRect`.
  - Easing curve: `QEasingCurve::OutCubic`, duration: `180ms`.
  - Trigger repaint on every step to ensure a silky-smooth window capture transition.

---

## 2. Advanced Annotation System

### 2.1 Drawing Engine
We will implement an annotation drawing stack in `ScreenshotWindow`.
- **Annotation Data Model**:
  ```cpp
  enum AnnotationType { AnnotateNone, AnnotateRect, AnnotateCircle, AnnotateArrow, AnnotatePen };
  struct Annotation {
      AnnotationType type;
      QRect rect;            // Used for bounding boxes of Rect, Circle, and Arrow
      QList<QPoint> points; // Used for Pen freehand strokes
      QColor color;
      int width;
  };
  ```
- **State Machine**:
  - When an annotation tool is active, dragging inside the selection draws the annotation instead of moving/resizing the crop rect.
  - Standard mouse handlers in `ScreenshotWindow` (`mousePressEvent`, `mouseMoveEvent`, `mouseReleaseEvent`) will switch between selection resizing/moving and annotation drawing.
- **Shapes & math**:
  - **Rectangle & Circle**: Drawn using standard `painter.drawRect()` and `painter.drawEllipse()`.
  - **Arrow**: Calculated dynamically. Draws a main shaft line and two angled arrowhead lines at the destination point:
    ```cpp
    double angle = std::atan2(end.y() - start.y(), end.x() - start.x());
    QPointF arrowP1 = end - QPointF(cos(angle + M_PI/6) * headLength, sin(angle + M_PI/6) * headLength);
    QPointF arrowP2 = end - QPointF(cos(angle - M_PI/6) * headLength, sin(angle - M_PI/6) * headLength);
    ```
  - **Pen**: Draws connecting anti-aliased segments from the points list.
- **Undo/Redo History**:
  - Keep `QList<Annotation> annotations;` and `QList<Annotation> undoHistory;` stacks for instant history traversal.

---

## 3. Interaction Polish

### 3.1 Outside-Click Quick Copy
- When `croppedRect` is active, clicking outside the selection area (and not on any handle or toolbar) will:
  1. Capture the `currentImage` (which is either the original crop or the translated/OCR-rendered image).
  2. Copy it to the clipboard using `QApplication::clipboard()->setPixmap()`.
  3. Close the screenshot window.

### 3.2 Premium Redesigned Settings Panel
- **Layout**:
  - Left Sidebar: Clean `QListWidget` styled as a vertical navigation bar with beautiful icons/text and elegant hover/selection states.
  - Right Side: `QStackedWidget` containing five cleanly structured pages:
    1. **服务与通道 (Services & Channels)**: N100 server URL, token, channel choice (new-api / Baidu / Google), API Keys, endpoints, and model puller.
    2. **本地 OCR 识别 (Local OCR)**: Toggle switch for Local OCR, engine path input, timeout spinbox, fallback toggle.
    3. **全局热键 (Global Hotkey)**: Global shortcut setup.
    4. **系统设置 (System Settings)**: Auto-start on boot toggle switch.
    5. **关于 (About)**: Simple app logo and details.
- **Cards Style**: Right-side controls grouped into modern, rounded white cards (`border-radius: 8px`, `border: 1px solid #f1f5f9`).

### 3.3 Animated Sliding Switch Toggles
- Custom widget `SwitchButton` (subclass of `QAbstractButton`).
- Uses `QVariantAnimation` to animate both:
  1. Background track color (smooth interpolation from `#e2e8f0` to `#3b82f6`).
  2. Sliding thumb offset (smooth interpolation of x-coordinate).
- Clicking results in a gorgeous micro-animation with high responsiveness.

### 3.4 Auto-Start on Boot (Windows Startup)
- Managed via registry at `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`.
- Standard read/write using `QSettings`. Adds/removes `"ScreenshotTranslator"` pointing to `QCoreApplication::applicationFilePath()`.

### 3.5 High-DPI Coordinate & Pixmap Scaling Fix (Perfect Alignment)
To eliminate misalignment and scaling distortion on High-DPI screens (e.g. 125%, 150%, 200% zoom) which causes layout shifting shown in Fig 1, we will standardise high-DPI scaling across all windowing, cropping, and rendering pipelines:
- **Set QPixmap Device Pixel Ratio**: Upon successfully receiving the translated or OCR-rendered image from the server, we will explicitly configure its device pixel ratio:
  ```cpp
  currentImage.setDevicePixelRatio(devicePixelRatio());
  ```
  This ensures Qt recognizes the image's physical dimensions (e.g., 300x300 pixels) at the correct logical scale (e.g., 200x200 pixels under 1.5x zoom), allowing `painter.drawPixmap` and `PinWindow` to draw it perfectly aligned, sharp, and pixel-perfect.
- **Scale OCR Results by DPI Ratio**: OCR coordinates returned from the backend (PaddleOCR is run on physical pixels) must be divided by `devicePixelRatio()` to convert them into logical coordinates for text-selection in `PinWindow::applyOcrResults`:
  ```cpp
  qreal ratio = devicePixelRatio();
  int x1 = p1.at(0).toDouble() / ratio;
  int y1 = p1.at(1).toDouble() / ratio;
  int x3 = p3.at(0).toDouble() / ratio;
  int y3 = p3.at(1).toDouble() / ratio;
  ```
- **Scale Snapped Window Rectangles**: Ensure all window rectangles scanned from the Win32 API are divided by `devicePixelRatio()` in `EnumWindowsProc` to ensure coordinates and geometry are processed purely in logical pixels.

---

## 4. Verification Plan

### 4.1 Unit & Integration Testing
- Verify drawing math for Arrow calculations.
- Test settings serialization/deserialization under different toggle states.
- Run compilation in Release mode to guarantee zero compilation warnings.

### 4.2 Manual Verification
- **Window Snapping**: Run various apps (including UWP and standard Win32 apps), check alignment, and verify the highlight snapping transition is smooth.
- **Annotations**: Draw multiple rectangles, circles, arrows, and brush strokes. Verify styling updates in real time and that Undo/Redo works flawlessly.
- **Outside Copy**: Draw a selection box, click outside, and verify the image has been copied to the clipboard and the screenshot window has closed.
- **Autostart**: Toggle "Start on boot" in the Settings panel and verify that the registry key is successfully updated.
