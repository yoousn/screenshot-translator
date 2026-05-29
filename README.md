# YSN 截图翻译 v1.0 (全新全栈桌面应用)

这是一个全新重构的 **全栈截图翻译系统**，支持 Windows 客户端与 N100 服务器，具备高性能本地处理 + 云端翻译能力，保持轻量启动与秒级响应。

---

## 🌟 核心特点

* **轻量桌面客户端**：约 15MB，无需 CUDA/Python，秒开即用。
* **高性能后端服务**：FastAPI + AI 翻译 + OCR，集中处理文本识别与翻译逻辑。
* **零延迟截图翻译**：选区拖拽 → OCR → 翻译 → 原地无痕嵌字重绘。
* **钉图 & 导出**：翻译结果可置顶钉图、复制或保存高清截图。
* **内外网双通**：支持内网直连与外网安全域名访问 N100。

---

## 📂 项目结构

```text
d:\Desktop\自制截图\
├── tauri-client/             # Windows React + Tauri 前端 + 本地后端
│   ├── src/                  # UI 与交互逻辑
│   ├── src-tauri/src/lib.rs  # Tauri 后端逻辑
│   └── config.json           # 本地配置文件 (或 yaml)
├── server/                   # N100 FastAPI 服务端
│   ├── app.py                # 服务入口
│   ├── translator.py         # 翻译引擎
│   ├── image_processor.py    # OCR 与图像处理
│   └── tests/                # 单元测试
└── docs/                     # 文档说明
```

---

## 🛠️ 部署与使用

### 1. N100 服务端

#### 环境

* Python 3.10+
* 依赖包：`fastapi`, `uvicorn`, `requests`, `pyyaml`, `numpy`, `pillow`, `paddleocr`, `opencv-contrib-python`
> **⚠️ 依赖提醒：** `paddleocr` 请固定安装 2.x 版本系列，以防未来 3.x 版本升级带来的模型接口与结构变更导致无法加载默认的 PP-OCRv4。

#### 启动

```bash
cd server
.venv/Scripts/python -m uvicorn app:app --host 0.0.0.0 --port 8318 --reload
```

服务端默认配置：`~/.screenshot-translator/config.yaml`。首次启动时会自动生成随机 `client_token` 并打印到控制台，请将其填入客户端设置。也可通过环境变量 `SS_TRANSLATOR_TOKEN` 覆盖。

---

### 2. Windows 客户端

#### 编译运行

* 使用 Tauri + React，直接 `npm install && npm run tauri dev` 启动开发模式
* 打包可生成单文件桌面应用

#### 核心功能

1. **后台运行**：启动后缩小到系统托盘。
2. **触发截图**：托盘图标双击或菜单触发。
3. **选区翻译**：拖动选区，Ctrl+Q 或翻译按钮进行 OCR + 翻译 + 无痕嵌字。
4. **复制 / 保存**：Ctrl+C 复制，Ctrl+S 保存高清嵌字图。
5. **钉图 (Pin)**：翻译结果可置顶为独立悬浮窗口。
6. **设置面板**：实时拉取模型、验证翻译账户连通性。

---

## 🖥️ N100 服务器运维

保留原有 N100 内外网访问与 SSH 自动化部署方式：

* 内网直连：`http://192.168.1.3:8318`
* 外网访问：`https://ocr.yousn.me`

更新后端代码后，按原有 SCP + SSH 自动化脚本部署，无需手动操作。

---

## 🔗 链接

* N100 服务器访问方式保留旧版链接风格：`http://192.168.1.3:8318` / `https://ocr.yousn.me`
