# YSN 截图翻译 (SaaS-like Screenshot Translator)

这是一个受 **PixPin** 启发的、专为 N100 服务器和 Windows 客户端设计的“极致轻量级”截图翻译系统。

## 🌟 核心设计架构

* **极轻量客户端**：保持在约 10MB ~ 20MB，秒开、秒运行，本地不需要安装 CUDA、Python、深度学习框架、OCR 包等。
* **高内聚服务端**：所有复杂的 AI 推理（PaddleOCR）、网络翻译、以及图形像素处理（OpenCV 多边形环形中位数背景颜色采样、无痕擦除、Pillow 动态字号折行嵌字）全都在 N100 集中处理，客户端只通过网络接口进行 HTTPS 渲染。
* **内外网双通**：内网直连（<5ms 延迟，支持 `http://192.168.1.3:8318`），外网通过安全域名（`https://ocr.yousn.me`）鉴权访问。

---

## 📂 项目结构

```text
d:\Desktop\自制截图\
├── docs/                     # 设计与实施规格说明书
├── server/                   # Python 服务端目录
│   ├── app.py                # FastAPI 服务入口
│   ├── config.py             # 统一 YAML 配置文件持久化读写
│   ├── translator.py         # 翻译引擎（Google Web 免签、new-api 兼容、百度翻译）
│   ├── image_processor.py    # OCR+背景采样涂抹+PIL嵌字重绘核心类
│   └── tests/                # 基于 pytest 的 TDD 单元测试用例
└── client/                   # Windows C++/Qt 客户端目录
    ├── CMakeLists.txt        # CMake 现代构建配置 (支持 Qt5/Qt6)
    ├── main.cpp              # 系统托盘双击激活截图后台入口
    ├── config.h/cpp          # 客户端本地 config.json 读取
    ├── networkclient.h/cpp   # API 网络请求与中转组件
    ├── screenshotwindow.h/cpp# 多屏幕蒙版框选、悬浮工具栏、翻译加载、钉图(Pin)组件
    └── settingspanel.h/cpp   # 精美配置面板 (含“拉取模型”、“连通性验证”等逻辑)
```

---

## 🛠️ 部署与使用方法

### 1. 服务端 (N100 Server)

#### 运行环境
* Python 3.10+
* 依赖包：`fastapi`, `uvicorn`, `requests`, `pyyaml`, `numpy`, `pillow`, `paddlepaddle`, `paddleocr`, `opencv-contrib-python`, `python-multipart`

#### 部署启动
在 `server` 目录下运行：
```bash
# 激活虚拟环境 (Windows/飞牛 OS 容器)
.venv/Scripts/python -m uvicorn app:app --host 0.0.0.0 --port 8318 --reload
```
* **服务端默认配置文件位置**：`~/.screenshot-translator/config.yaml`。默认带有 `client_token`: `ysn-screenshot-translator-token-666` 安全校验。

#### 单元测试运行
```bash
# 运行全部测试 (包含 Google翻译, PaddleOCR 颜色采样, FastAPI 路由与鉴权)
$env:PYTHONPATH="server"; .venv/Scripts/python -m pytest
```

---

### 2. 客户端 (C++/Qt 6 Client)

#### 编译准备
* **C++ 编译器**：支持 C++17 的编译器 (如 MSVC 2019+, GCC)
* **CMake**：3.16+
* **Qt 库**：Qt 5 或 Qt 6 (包含 `Core`, `Gui`, `Widgets`, `Network` 模块)

#### 编译与运行
使用 Qt Creator 直接打开 `client/CMakeLists.txt`，配置并点击 **Run** 编译运行。

#### 客户端核心交互功能
1. **后台运行**：启动后程序自动缩小到系统托盘，并弹出气泡提示。
2. **触发截图**：**双击托盘图标** 或 **单击托盘菜单中的“截图翻译”** 即可进入截图状态。
3. **选区拖动**：按住左键拖动框选区域，松开后在选区下方弹出悬浮工具栏。
4. **翻译与重绘 (Ctrl+Q)**：点击悬浮工具栏的 `翻译` 按钮或按快捷键 `Ctrl+Q`。选区蒙版会自动高亮“正在翻译与无痕嵌字中...”，等待约 1 秒后，被抹除文字并完美居中重绘了汉字的翻译结果图就会原地替换呈现！
5. **复制 (Ctrl+C) / 保存 (Ctrl+S)**：获取最终处理后的高清晰度嵌字图片。
6. **钉图 (Pin)**：点击 `钉图` 会将翻译后的截图作为独立的无边框悬浮窗口置顶钉在桌面上，支持左键拖动位置，右键菜单进行复制或关闭。
7. **设置面板**：服务器和中转地址上方均有内网的浅灰色 Hint 字样引导。
   * 点击 **“获取模型”**：客户端会向 N100 发起模型提取代理，实时拉取 new-api 的所有可用模型，自动更新下拉框。
   * 点击 **“点击验证”**：对当前填入的所有翻译账户进行在线连通性验证。若通畅，后端会将此通道直接锁定为当前激活的主力翻译器并完成持久化，客户端右侧会回显绿色的“验证成功”字样！

---

## 🖥️ N100 服务器运维与部署自动化 (SSH & SCP Guide for Agents)

为了让未来的 AI 助手或维护者无需任何外部提示即可瞬间连接服务器并完成部署，以下是 N100 服务器的详细运维资料：

### 1. 连接凭证清单 (Credentials)
* **SSH 密钥**：本地路径为 `~/.ssh/n100_key` (Windows 主机下为 `C:\Users\Administrator\.ssh\n100_key`)
* **连接命令**：可以通过本地 SSH 别名 `ssh n100` 或执行直连命令：
  * **公网 SSH（穿透）**：`ssh -i ~/.ssh/n100_key -p 56001 ysn@47.76.135.185`
  * **内网 SSH**：`ssh -i ~/.ssh/n100_key -p 22 ysn@192.168.1.3`
* **服务端项目目录**：`/vol1/1000/项目/自制截图/server`
* **服务端端口**：`8318`

### 2. 自动化代码同步与重启服务 (Deploy Commands)
当更新了 `server/` 文件夹的代码（如 `image_processor.py`、`app.py`）后，在**本地控制台**依次执行以下 PowerShell 命令即可实现“一键同步与热重载”：

```powershell
# A. 上传最新代码文件到 N100
scp -i "$env:USERPROFILE\.ssh\n100_key" -P 56001 -o StrictHostKeyChecking=no server/image_processor.py ysn@47.76.135.185:/vol1/1000/项目/自制截图/server/image_processor.py
scp -i "$env:USERPROFILE\.ssh\n100_key" -P 56001 -o StrictHostKeyChecking=no server/app.py ysn@47.76.135.185:/vol1/1000/项目/自制截图/server/app.py

# B. 强杀 8318 端口旧进程，等待 3 秒释放 socket 后，后台静默启动新版服务
ssh -i "$env:USERPROFILE\.ssh\n100_key" -p 56001 -o StrictHostKeyChecking=no ysn@47.76.135.185 "fuser -k 8318/tcp; sleep 3; cd /vol1/1000/项目/自制截图/server && nohup .venv/bin/python -m uvicorn app:app --host 0.0.0.0 --port 8318 > uvicorn.log 2>&1 &"

# C. 验证服务是否成功启动并监听
ssh -i "$env:USERPROFILE\.ssh\n100_key" -p 56001 -o StrictHostKeyChecking=no ysn@47.76.135.185 "tail -n 20 /vol1/1000/项目/自制截图/server/uvicorn.log"
```

