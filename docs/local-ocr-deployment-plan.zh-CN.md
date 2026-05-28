# 本地 OCR 部署方案（AI 实施版）

本文档给后续 AI / 开发者使用，目标是把 OCR 从 N100 服务端迁移到 Windows 客户端本地执行，N100 只负责鉴权、翻译、缓存、后台配置与兜底服务。

## 1. 背景与目标

当前链路：

```text
Windows 客户端 EXE
  ↓ 上传选区图片
https://ocr.yousn.me
  ↓
N100: FastAPI + PaddleOCR + 翻译 + 图片重绘
```

实测瓶颈：

```text
Translate Step: 0.03 ms
Cache Hits: 5
OCR Step: 1054.81 ms
```

`pidstat` 显示 N100 在 OCR 时 Python 进程可达到 82%~87% CPU；N100 是 4 CPU，Linux 下 100% 约等于 1 个核心满载，因此当前核心瓶颈是 PaddleOCR 在 N100 上受单核/少线程性能限制。

目标链路：

```text
Windows 客户端 EXE
  ├─ 截图选区
  ├─ 本地 OCR
  ├─ 本地图片重绘
  ↓ 只发送文本/坐标/token
https://ocr.yousn.me
  ├─ 鉴权
  ├─ 文本翻译
  ├─ 翻译缓存
  ├─ 用户配置/额度/日志
  └─ 云端 OCR 兜底
```

核心目标：

```text
保持 ocr_max_side = 1280 或更高
减少 N100 OCR 延迟
保留云端统一鉴权和翻译通道管理
客户端平时低内存，OCR 按需启动
```

## 2. 设计原则

```text
截图功能保持轻量
OCR 模型不随主界面常驻加载
OCR 进程按需启动，空闲自动退出
N100 不再默认承担 OCR 主路径
服务端保留 /api/translate 作为兜底兼容接口
新增 /api/translate_text 作为本地 OCR 后的主接口
```

推荐运行模式：

```text
默认：本地 OCR 开启
失败：自动回退云端 /api/translate
调试：允许切换 serverUrl
生产：隐藏 serverUrl，固定 https://ocr.yousn.me
```

## 3. 方案选择

### 3.1 方案 A：PaddleOCR-json 本地子进程（优先推荐）

特点：

```text
Windows 集成简单
可通过 stdin/stdout 或 HTTP/命令行调用
不需要在 Tauri 主进程内嵌 Python
部署时作为外部 runtime 放入客户端资源目录
```

适合当前项目，因为现有设置页已经存在：

```text
useLocalOcr
fallbackToRemoteOcr
localOcrExecutablePath
localOcrTimeoutMs
```

需要补齐：

```text
Rust/Tauri 命令 run_local_ocr
截图页 ScreenshotPage.tsx 本地 OCR 调用逻辑
N100 /api/translate_text 文本翻译接口
客户端本地重绘逻辑
```

### 3.2 方案 B：RapidOCR + ONNX Runtime

特点：

```text
可选 CPU / DirectML / CUDA / OpenVINO 后端
模型体积和启动速度可能优于完整 PaddleOCR Python 环境
工程集成复杂度中等
```

适合后续优化版，不作为第一版落地优先项。

### 3.3 方案 C：继续 N100 OCR + 降低 ocr_max_side

不采用为主方案，因为用户明确希望保持 1280，甚至后续提高识别清晰度。

### 3.4 方案 D：GPU 服务器 OCR

效果最好但成本最高，不适合作为当前第一步。

## 4. 推荐架构

```text
Tauri 前端 ScreenshotPage.tsx
  ↓ capture_region
Tauri Rust lib.rs
  ↓ 保存选区图片到临时文件
LocalOcrManager
  ↓ 启动/复用 PaddleOCR-json.exe
  ↓ 返回 OCR blocks: text + box + confidence
ScreenshotPage.tsx
  ↓ POST /api/translate_text
N100 FastAPI app.py
  ↓ 翻译 + 缓存
ScreenshotPage.tsx
  ↓ 本地重绘译文到 canvas/PNG
```

## 5. 新增数据结构

### 5.1 OCR block

```ts
interface OcrBlock {
  text: string;
  confidence: number;
  box: [number, number][];
}
```

### 5.2 翻译文本请求

```ts
interface TranslateTextRequest {
  blocks: OcrBlock[];
  source_lang?: string;
  target_lang?: string;
  render_mode?: "client";
}
```

### 5.3 翻译文本响应

```ts
interface TranslateTextResponse {
  status: "success" | "failed";
  translations: string[];
  cache_hits: number;
  channel: string;
  error?: string;
}
```

## 6. 服务端改造

### 6.1 新增接口

文件：`server/app.py`

新增：

```text
POST /api/translate_text
```

请求头：

```text
x-api-key: client token
```

请求体：

```json
{
  "blocks": [
    {
      "text": "Hello world",
      "confidence": 0.98,
      "box": [[0,0],[100,0],[100,30],[0,30]]
    }
  ],
  "source_lang": "auto",
  "target_lang": "zh"
}
```

响应：

```json
{
  "status": "success",
  "translations": ["你好，世界"],
  "cache_hits": 1,
  "channel": "google"
}
```

### 6.2 服务端职责

```text
校验 token
读取 active_channel
调用 translator.translate_batch
返回译文数组
不处理图片
不运行 OCR
不重绘图片
```

### 6.3 保留旧接口

保留：

```text
POST /api/translate
POST /api/ocr
```

用途：

```text
旧客户端兼容
本地 OCR 失败时回退
调试 N100 OCR
```

## 7. 客户端 Rust/Tauri 改造

文件：`tauri-client/src-tauri/src/lib.rs`

### 7.1 新增命令

```rust
#[tauri::command]
async fn run_local_ocr(image_base64: String, executable_path: Option<String>, timeout_ms: Option<u64>) -> Result<serde_json::Value, String>
```

### 7.2 执行逻辑

```text
base64 解码
写入 app_data_dir/local_ocr_input.png
查找 OCR 可执行文件：
  1. config.localOcrExecutablePath
  2. bundled resources/ocr/PaddleOCR-json.exe
  3. PATH
启动 OCR 子进程
传入图片路径
设置 timeout
解析 stdout JSON
统一转换为 OcrBlock[]
失败返回 Err
```

### 7.3 进程管理

第一阶段：每次 OCR 启动一次进程。

第二阶段：实现常驻管理：

```text
首次 OCR 启动子进程
连续请求复用
5 分钟无任务自动退出
客户端退出时 kill 子进程
```

推荐先做第一阶段，验证准确率和链路后再做常驻。

## 8. 客户端前端改造

文件：`tauri-client/src/pages/ScreenshotPage.tsx`

### 8.1 新流程

```text
handleTranslate()
  ↓
读取 config.useLocalOcr
  ↓
如果 true：
  captureRegionBase64()
  invoke("run_local_ocr")
  fetch(`${serverUrl}/api/translate_text`)
  drawTranslatedBlocks()
  setTranslatedResult()
  ↓
如果失败且 fallbackToRemoteOcr=true：
  invoke("api_translate")
```

### 8.2 兼容旧流程

```text
useLocalOcr=false
  → 继续走 api_translate
```

### 8.3 本地重绘函数

新增：

```ts
function renderTranslatedBlocks(base64Image: string, blocks: OcrBlock[], translations: string[]): Promise<string>
```

职责：

```text
创建 canvas
绘制原选区图片
根据 box 取背景色
覆盖原文字区域
绘制译文
输出 PNG base64
```

第一版可以复用当前服务端 image_processor.py 的策略：

```text
采样背景色
按 box 高度计算字体
自动换行
文字颜色根据背景明暗选择黑/白
```

## 9. 设置页改造

文件：`tauri-client/src/pages/Settings.tsx`

保留并真正启用：

```text
useLocalOcr
fallbackToRemoteOcr
localOcrExecutablePath
localOcrTimeoutMs
```

新增推荐默认值：

```json
{
  "useLocalOcr": true,
  "fallbackToRemoteOcr": true,
  "localOcrTimeoutMs": 8000
}
```

生产版建议：

```text
隐藏 serverUrl
固定 https://ocr.yousn.me
只允许输入 token
高级设置里才显示 serverUrl
```

## 10. 打包部署

### 10.1 目录建议

```text
tauri-client/
  src-tauri/
    resources/
      ocr/
        PaddleOCR-json.exe
        models/
          det/
          rec/
          cls/
```

### 10.2 Tauri 配置

文件：`tauri-client/src-tauri/tauri.conf.json`

增加资源打包配置，确保 OCR runtime 被包含进安装包。

### 10.3 客户端启动后路径解析

优先级：

```text
用户自定义 localOcrExecutablePath
内置 resource 目录
PATH 环境变量
```

## 11. 性能目标

当前 N100：

```text
OCR Step: 100~1200ms
Translate Step 命中缓存后: 0.01~0.03ms
```

目标 Windows 本地 CPU：

```text
R5 7500F CPU OCR: 80~300ms
RTX 4060 GPU OCR: 20~100ms
```

可接受指标：

```text
首次 OCR: < 1500ms
常驻后 OCR: < 300ms
翻译缓存命中后总耗时: < 500ms
失败自动回退云端，不影响主流程
```

## 12. 内存策略

当前客户端：

```text
40~80MB
```

本地 OCR 后：

```text
不常驻 OCR：平时 40~80MB，识别时临时 300MB~1GB
常驻 OCR：平时 300MB~1GB
```

推荐：

```text
默认不常驻
连续使用时保活 5 分钟
空闲自动退出
```

## 13. 安全与隐私

优势：

```text
图片不再默认上传 N100
N100 只接收 OCR 文本和坐标
客户隐私更好
带宽更低
```

注意：

```text
token 不写死进代码
生产版隐藏 serverUrl
日志不要输出完整敏感文本
OCR runtime 不允许执行任意路径参数
```

## 14. 测试清单

### 14.1 功能测试

```text
本地 OCR 开启：截图翻译成功
本地 OCR 关闭：云端翻译成功
本地 OCR exe 路径错误：自动回退云端
token 错误：返回 401 并显示友好错误
serverUrl 不通：显示网络错误
```

### 14.2 性能测试

```text
同一区域连续截图 5 次
记录 OCR 耗时
记录翻译耗时
记录总耗时
记录客户端内存
记录 OCR 子进程是否自动退出
```

### 14.3 准确率测试

```text
英文网页小字
软件菜单文字
中文界面
中英混排
高 DPI 屏幕
多显示器
深色背景
浅色背景
```

## 15. 实施里程碑

### Milestone 1：服务端文本翻译接口

```text
新增 /api/translate_text
复用 translator.translate_batch
返回 translations/cache_hits/channel
```

### Milestone 2：客户端本地 OCR 命令

```text
Rust 新增 run_local_ocr
能调用外部 OCR exe
能返回标准 OcrBlock[]
```

### Milestone 3：截图页接入

```text
handleTranslate 支持 useLocalOcr
本地 OCR 成功后调用 /api/translate_text
失败后回退 /api/translate
```

### Milestone 4：本地重绘

```text
在客户端 canvas 内根据 OCR box 重绘译文
生成 translatedResult base64
复制/保存逻辑不变
```

### Milestone 5：打包

```text
OCR runtime 放入 Tauri resources
安装后无需用户手动配置 OCR exe
```

## 16. 回滚方案

如果本地 OCR 方案出现问题：

```text
useLocalOcr=false
fallbackToRemoteOcr=true
恢复旧链路 /api/translate
```

不删除 N100 OCR 能力，直到本地 OCR 稳定。