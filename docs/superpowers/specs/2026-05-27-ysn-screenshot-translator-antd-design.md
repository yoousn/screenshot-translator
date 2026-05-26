# YSN 截图翻译客户端 UI 重构设计规范

本项目旨在对 YSN 截图翻译客户端的前端进行重构，全部基于 **Ant Design v5** 与 **React + TypeScript**。目标是实现一套具有企业级桌面工具感、统一规格、高度交互的真实 UI，并完全修复截图与状态检测等 P0 核心功能。

---

## 1. 界面与布局设计 (Layout & Components)

### 1.1 全局布局架构 (App Layout)
* **Sider (左侧导航栏)**：宽度固定为 `200px`，白色背景，与右侧通过细分割线区分。
  * **Brand Header**：高度为 `56px`。包含精致的 `Y` 渐变圆角图标和“YSN 截图翻译”字样。
  * **Menu (导航菜单)**：支持四个板块：控制面板（Dashboard）、系统设置（Settings）、历史记录（History）、关于系统（About）。
  * **Footer Action (立即截图)**：常驻一个 Primary 按钮，高度统一为 `36px`，点击时调用 Tauri Rust 命令 `start_screenshot`。
* **Header (顶部状态栏)**：高度固定为 `56px`，白色背景。
  * **算法说明**：展示 `算法核心：PIL 图像像素引擎 + PaddleOCR` 提示信息。
  * **服务状态监测**：右侧展示服务器状态（在线 `Online` / 离线 `Offline` / `检测中`）的 `Tag`，悬浮时使用 `Tooltip` 显示服务器 URL。提供刷新按钮用于手动检查并带有 `loading` 状态。
* **Content (主内容区)**：背景色统一为 `#f5f7fb`，四周留白 `24px`。

### 1.2 控制面板 (Dashboard)
* 头部展示“控制面板”标题及说明。
* 内部使用 `Tabs` 分类：
  * **截图功能**：展示功能列表行。
    * **立即截图**：绑定真实截图函数，点击立即调用 `start_screenshot` 命令。
    * **延迟截图、固定贴图 (Pin)、提取文本 (OCR)、自动翻译、一键复制、截取全屏、截取窗口**：这些功能尚未实现，全部设为 `disabled`，当鼠标 Hover 时，使用 `Tooltip` 显示“开发中”，确保绝不出现假装可用、无响应的假按钮。
    * 功能行内的快捷键 `Tag`、状态及操作按钮的尺寸在高度上完美对齐（按钮高度 32px，行高 56px）。
  * **接口测试**：
    * 包含三张卡片（Card，圆角 `12px`）：服务器连接状态、翻译信道（谷歌/百度/大模型）、延迟响应时间。
    * 拖拽图片测试模块（Drag-and-Drop Image Tester）：使用 `Upload.Dragger`。支持拖拽或选择本地图片，点击“开始翻译”后调用后端接口，右侧显示翻译重绘后的结果预览（提供“保存图片”下载链接）。

### 1.3 系统设置 (Settings)
* 将原有组件重构为标准的 Ant Design `Form` 表单：
  * **后端服务配置**：API 服务器地址（如 `https://ocr.yousn.me`），客户端认证令牌（Token，密码框）。
  * **翻译信道配置**：下拉菜单 `Select` 切换“谷歌翻译（免密）”、“百度翻译（开放平台）”、“中转大模型（New API）”。
    * 切换为百度翻译时，表单动态展开 App ID 和密钥输入框，提供“测试连接并启用”按钮（有 `loading` 状态）。
    * 切换为中转大模型时，展开中转地址、API Key 输入框，并可点击“拉取模型”获取可用模型列表。
  * **本地 OCR**：使用 `Switch` 开关控制“启用本地 OCR”和“自动回退到云端 OCR”。提供本地可执行程序物理路径输入框与超时时间输入框（InputNumber）。
  * **系统控制**：开机自动启动（`Switch`）、全局截图快捷键。
* 顶部表单操作行高 48px，右侧包含“保存设置” Primary 按钮（高度 36px），点击时通过 Form 触发保存逻辑。

### 1.4 历史记录 (History) 与 关于 (About)
* **历史记录**：展示精美的历史翻译流水线列表，右侧对齐展示识别块个数、翻译耗时和状态 Tag，右上角提供“清理历史记录”按钮。
* **关于系统**：展示 YSN 截图翻译系统介绍，展示 Tauri 2.0 优势和各开源项目的致谢，下方附带 GitHub 链接。

---

## 2. 核心功能及 Rust 命令关联

### 2.1 截图命令 (`start_screenshot`)
* **逻辑**：点击“立即截图”时，调用 Tauri 命令 `start_screenshot`。
* **反馈**：使用 `message.loading` 提示，并在成功或失败时分别触发 `message.success` 或 `message.error` 报错。

### 2.2 状态监测 (`/api/health`)
* 对服务器 URL 进行异步 Fetch 请求，路由为 `/api/health`。
* 支持在加载时和手动点击刷新按钮时重新触发，刷新按钮带有 loading。

---

## 3. TypeScript 编译与构建

* **TypeScript 修复**：
  1. 将所有 `<Text bold>` 替换为 `<Text strong>` 或使用 `style={{ fontWeight: "bold" }}` 以符合 Antd Typography.Text API 规范。
  2. 在 `Dashboard.tsx` 顶部导入 `SyncOutlined` 图标。
* **冗余文件清理**：
  * 在项目打包及最终交付前，删除不需要的旧版/冗余构建产物或静态演示资源，保持 workspace 整洁。
* **Windows 可执行文件构建**：
  * 编译 Tauri 项目，生成最终的 Windows 免安装可执行程序，以确保程序能直接在物理机上完美启动。

---

## 4. 交付硬约束

1. **不允许假 UI**
   - 所有可点击按钮必须绑定真实函数
   - 暂未实现的功能必须 disabled
   - 禁止点击无反应
2. **截图功能必须作为 P0 验收**
   - “立即截图”必须实际触发截图流程
   - 若 `start_screenshot` 尚未实现，必须先实现 Rust command 或桥接旧客户端截图能力
   - 不能只显示 message.success
3. **Ant Design 必须真实接入**
   - 所有 Button / Input / Select / Switch / Tabs / Card / Menu 必须来自 antd
   - 禁止用 div/span 模拟组件
   - 禁止继续使用浏览器默认控件样式
4. **每次提交必须给出验证结果**
   - npm run build
   - npm run tauri build 或 cargo check
   - 截图按钮实测结果
   - /api/health 实测结果
   - UI 截图

