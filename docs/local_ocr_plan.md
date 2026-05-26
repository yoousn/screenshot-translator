# 本地 OCR 升级开发计划 (Local OCR Implementation Plan)

这是一个用于将《自制截图翻译》项目升级为具有 **“可选本地 OCR、低延迟识别、云端回退”** 能力的技术改造计划。

当前阶段只实现 **本地 OCR 识别**：框选/钉图后的文字检测与识别可由 Windows 本地 `PaddleOCR-json` 完成；**翻译、无痕擦除、嵌字重绘仍沿用现有服务端链路**。因此本阶段目标不是完整 100% 离线翻译，而是先把 OCR 从云端拆出来，降低延迟并支持断网识别文字。

我们将采用 **路线 A（集成 C++ `PaddleOCR-json` 后台服务进程）** 进行快速实现。如果路线 A 遇到无法解决的环境或性能瓶颈，我们将启用 **路线 B（手写 ONNX Runtime 嵌入）**。

---

## 📅 版本进度追踪 (Progress Tracker)

| 阶段 | 任务目标 | 状态 | 耗时/更新时间 | 备注 |
| :--- | :--- | :--- | :--- | :--- |
| **第 0 阶段** | **准备工作**：梳理本地 OCR 技术路线，调试解决当前云端服务乱码与 API 调用异常。 | ✅ 已完成 | 2026-05-26 | 已建立本计划书，修复了 Linux 服务器端中文字体缺失问题，并在服务器端增加了 API 调用埋点，成功实现 N100 一键同步部署。 |
| **第 1 阶段** | **引入组件**：获取并配置适用于 Windows 的免安装绿色版 `PaddleOCR-json` 引擎，放至客户端发布目录。 | ✅ 代码侧完成 | 2026-05-26 | 已修正计划范围：本阶段实现可选本地 OCR，不承诺完整离线翻译/嵌字；引擎二进制仍需用户下载安装到本地路径。 |
| **第 2 阶段** | **主程序集成**：在 C++ 中设计 `LocalOCRManager` 类，通过 `QProcess` 建立与 OCR 子进程的双管道通信，并提供远程 OCR 回退。 | ✅ 代码侧完成 | 2026-05-26 | 已新增本地 OCR 配置持久化、设置面板入口、`LocalOcrManager`、CMake 构建入口、钉图 OCR 本地优先与云端回退逻辑；已按 `PaddleOCR-json -image_path=...` 与 `{code:100,data:[...]}` 输出做适配。 |
| **第 3 阶段** | **联调测试**：打通“本地 OCR 识别 -> 在线翻译/重绘回退”的完整闭环，并验证断网 OCR、异常回退、超时不冻结 UI。 | ✅ 构建通过，待实机联调 | 2026-05-26 | 已定位 Qt 6.11.1 MinGW kit，并通过 ASCII-only 源码联接目录与构建目录成功构建 `ScreenshotTranslator.exe`；后续需接入实际 `PaddleOCR-json.exe` 验证本地 OCR、云端回退与 UI 行为。 |

---

## 🛠️ 路线 A 技术规格设计 (Route A Architecture)

### 1. 组件选型
* **本地 OCR 引擎**：[PaddleOCR-json](https://github.com/hiroi-sora/PaddleOCR-json)
  * **特点**：基于 C++ 编译的 PaddleOCR 封装，支持通过 TCP/匿名管道进行 JSON 交互。
  * **体量**：压缩包仅约 15MB 左右，内存占用极低。
  * **速度**：单张截图识别仅需 50ms - 150ms。

### 2. 交互逻辑
```
[C++/Qt 客户端]
       │
       ├─ (启动/首次 OCR) ──► 用 QProcess 启动后台 PaddleOCR-json.exe
       │
       ├─ (框选/钉图 OCR) ─► 将截图暂存为临时文件或内存 Base64 ──► 通过 stdin 写入子进程
       │
       ├─ (管道回传) ◄── 子进程返回 OCR JSON 数据 (包含文字和四角坐标)
       │
       ├─ (格式适配) ──► 转换为现有客户端可消费的 {status, ocr:[{box,text}]} 结构
       │
       └─ (翻译/嵌字) ──► 继续提交给现有云端/服务端翻译与图像重绘链路
```

### 3. 本阶段边界与回退策略
* **本阶段包含**：本地 OCR 开关、OCR 引擎路径配置、`QProcess` 子进程管理、OCR JSON 格式适配、超时/崩溃处理、远程 OCR 回退。
* **本阶段不包含**：本地翻译、本地图片擦除、本地嵌字重绘、完整离线翻译成图。
* **默认策略**：本地 OCR 默认关闭；用户启用后优先本地 OCR，若引擎缺失、启动失败、超时或返回格式异常，则在允许回退时继续调用现有 `/api/ocr`。
* **验收标准**：关闭本地 OCR 时旧功能不变；开启本地 OCR 后 OCR 失败不导致 UI 卡死；翻译流程仍能复用现有 `translateImage`。

### 4. 本轮实现结果 (2026-05-26)
* 已新增 `ClientConfig` 本地 OCR 字段：`useLocalOcr`、`localOcrExecutablePath`、`localOcrTimeoutMs`、`fallbackToRemoteOcr`，并写入 `config.json` 持久化。
* 已新增 `LocalOcrManager`：使用 `QProcess` 调用 `PaddleOCR-json.exe -image_path=<临时图片>`，收集 stdout/stderr，支持超时 kill、单请求防重入、临时文件显式清理。
* 已适配 OCR 输出：兼容现有服务端 `{status:"success", ocr:[...]}`，兼容 `PaddleOCR-json` `{code:100, data:[{text, box, score}]}`，并把 `code:101` 作为“无文字但成功”的空结果。
* 已在设置面板加入“本地 OCR 设置”：启用开关、引擎路径浏览、超时时间、失败回退云端 OCR；启用时校验必须是有效可执行绝对路径。
* 已在钉图 OCR 流程接入本地优先：开启本地 OCR 时优先本地识别，失败且允许回退时继续调用原 `/api/ocr`。
* 已修复代码审查中指出的关键问题：临时图片提前删除/未 flush、超时与 finished 双回调、并发请求状态覆盖、可执行路径校验不足、OCR box 形状校验不足。
* 已执行 `git diff --check`，无空白错误。
* 构建验证已通过：当前 Windows 环境 Qt kit 位于 `C:\Qt\6.11.1\mingw_64`，构建工具位于 `C:\Qt\Tools\Ninja\ninja.exe` 与 `C:\Qt\Tools\mingw1310_64\bin`。由于 Qt AutoMoc 在本机中文路径 `C:\Users\ysn\Desktop\自制截图` 下生成文件失败，采用 ASCII-only 目录联接 `C:\qt_screenshot_client` 指向 `client`，并输出到 `C:\qt_screenshot_build` 后成功构建 `ScreenshotTranslator.exe`。推荐构建命令：`cmake -S /c/qt_screenshot_client -B /c/qt_screenshot_build -G Ninja -DCMAKE_PREFIX_PATH="C:/Qt/6.11.1/mingw_64" -DCMAKE_MAKE_PROGRAM="C:/Qt/Tools/Ninja/ninja.exe" -DCMAKE_CXX_COMPILER="C:/Qt/Tools/mingw1310_64/bin/g++.exe" -DCMAKE_RC_COMPILER="C:/Qt/Tools/mingw1310_64/bin/windres.exe" -DCMAKE_BUILD_TYPE=Release`，随后执行 `cmake --build /c/qt_screenshot_build --config Release`。`git diff --check` 已通过。

---

## 📌 当前待解决痛点 (Current Backlog & Hotfixes)

1. **翻译结果乱码 (Tofu Boxes `□□□`)**：
   * **根本原因**：远程 N100 服务器（尤其是 Docker 容器或轻量 Linux 环境）中缺少支持中文 CJK 字符的字体文件。当 Pillow 重绘译文时，找不到字体，被迫回退到 Arial 从而渲染为 `□`。
   * **解决方案**：在服务器 `image_processor.py` 中引入**自愈式字体下载机制**，如果系统无中文字体，则自动从国内高速 CDN (jsDelivr) 极速下载文泉驿微米黑字体 (`wqy-microhei.ttc`) 到配置目录中自动载入。
   
2. **百度翻译 API 疑似未调用**：
   * **根本原因**：客户端本地的 `"channel": "baidu"` 配置仅在本地保存，如果用户没有在“配置面板”中点击**“点击验证”**，服务器端的 `active_channel` 配置文件没有同步刷新，仍默认走免费的 `google` 通道，因此百度后台统计没有增加。
   * **解决方案**：
     1. 指导用户进入配置面板点击一次“点击验证”使服务器写入配置。
     2. 在服务器 `app.py` 和 `translator.py` 中加入明显的**通道选择与调用日志埋点**。当用户发起翻译时，服务器会在终端控制台清晰打印 `[Active Translator] ...` 以及 `[BaiduTranslator] Translating ...`，让排查一目了然。

---

> [!NOTE]
> 本计划书已持久化在 [local_ocr_plan.md](file:///d:/Desktop/自制截图/docs/local_ocr_plan.md)，后续每一步改造我们都将在此同步更新，直至彻底完成！
