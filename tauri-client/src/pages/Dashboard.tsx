import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { 
  Tabs, 
  Card, 
  Button, 
  Tag, 
  Space, 
  Flex, 
  Row, 
  Col, 
  List, 
  Tooltip, 
  Upload, 
  message, 
  Typography,
  Empty
} from "antd";
import {
  CameraOutlined,
  ClockCircleOutlined,
  ScanOutlined,
  TranslationOutlined,
  CopyOutlined,
  DesktopOutlined,
  BorderInnerOutlined,
  InboxOutlined,
  SaveOutlined,
  WifiOutlined,
  GlobalOutlined,
  DashboardOutlined,
  RightOutlined,
  SyncOutlined
} from "@ant-design/icons";

const { Text, Title, Paragraph } = Typography;

interface Config {
  serverUrl?: string;
  clientToken?: string;
  channel?: string;
  useLocalOcr?: boolean;
  hotkey?: string;
}

interface DashboardProps {
  onStartScreenshot: () => void;
  shortcutError?: string | null;
}

export default function Dashboard({ onStartScreenshot, shortcutError }: DashboardProps) {
  const [config, setConfig] = useState<Config>({});
  const [serverStatus, setServerStatus] = useState<"checking" | "online" | "offline">("checking");
  const [responseTime, setResponseTime] = useState<number | null>(null);
  
  // Translation tester states
  const [isTranslating, setIsTranslating] = useState(false);
  const [translatedImage, setTranslatedImage] = useState<string | null>(null);
  const [dragActive, setDragActive] = useState(false);
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);

  // Delayed screenshot state
  const [delayedCountdown, setDelayedCountdown] = useState<number | null>(null);
  const [delayedActive, setDelayedActive] = useState(false);

  useEffect(() => {
    loadConfig();

    const unlistenPromise = listen<string>("screenshot-captured", (event) => {
      try {
        const base64Data = event.payload;
        const byteCharacters = atob(base64Data);
        const byteNumbers = new Array(byteCharacters.length);
        for (let i = 0; i < byteCharacters.length; i++) {
          byteNumbers[i] = byteCharacters.charCodeAt(i);
        }
        const byteArray = new Uint8Array(byteNumbers);
        const blob = new Blob([byteArray], { type: "image/png" });
        const file = new File([blob], "screenshot.png", { type: "image/png" });
        
        setSelectedFile(file);
        setPreviewUrl(URL.createObjectURL(file));
        setTranslatedImage(null);
        message.success("获取原生选区截图成功！可在下方进行连通性测试。");
      } catch (err) {
        console.error("处理截图事件失败", err);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  // Delayed screenshot countdown effect
  useEffect(() => {
    if (!delayedActive || delayedCountdown === null) return;
    if (delayedCountdown <= 0) {
      setDelayedCountdown(null);
      setDelayedActive(false);
      onStartScreenshot();
      return;
    }
    const timer = setTimeout(() => {
      setDelayedCountdown(prev => (prev !== null ? prev - 1 : null));
    }, 1000);
    return () => clearTimeout(timer);
  }, [delayedCountdown, delayedActive]);

  const handleDelayedScreenshot = () => {
    setDelayedCountdown(3);
    setDelayedActive(true);
    message.info("3 秒倒计时开始，请准备好要截取的内容...");
  };

  const handleFullscreenCapture = async () => {
    try {
      await invoke("quick_fullscreen_capture");
      message.success("全屏截图已复制到剪贴板");
    } catch (e: any) {
      message.error("全屏截图失败: " + e.toString());
    }
  };

  const loadConfig = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(configStr);
      setConfig(parsedConfig);
      checkServer(parsedConfig.serverUrl);
    } catch (error) {
      console.error("Failed to load config:", error);
    }
  };

  const checkServer = async (url?: string) => {
    const targetUrl = url || config.serverUrl || "https://ocr.yousn.me";
    setServerStatus("checking");
    const start = performance.now();
    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 4000);
      
      const response = await fetch(`${targetUrl.replace(/\/$/, "")}/api/health`, {
        method: "GET",
        signal: controller.signal,
      });
      
      clearTimeout(timeoutId);
      
      if (response.ok) {
        setServerStatus("online");
        setResponseTime(Math.round(performance.now() - start));
      } else {
        setServerStatus("offline");
      }
    } catch (e) {
      setServerStatus("offline");
    }
  };

  const handleCustomUpload = (options: any) => {
    const file = options.file as File;
    if (file.type.startsWith("image/")) {
      setSelectedFile(file);
      setPreviewUrl(URL.createObjectURL(file));
      setTranslatedImage(null);
      options.onSuccess();
    } else {
      message.error("只能上传图片文件");
      options.onError();
    }
  };

  const startTranslation = async () => {
    if (!selectedFile) return;
    setIsTranslating(true);
    
    const targetUrl = config.serverUrl || "https://ocr.yousn.me";
    const token = config.clientToken || "";
    
    const formData = new FormData();
    formData.append("image", selectedFile);
    
    try {
      const response = await fetch(`${targetUrl.replace(/\/$/, "")}/api/translate`, {
        method: "POST",
        headers: {
          "x-api-key": token
        },
        body: formData,
      });
      
      if (!response.ok) {
        const errText = await response.text();
        throw new Error(errText || `HTTP 错误 ${response.status}`);
      }
      
      const blob = await response.blob();
      const imageUrl = URL.createObjectURL(blob);
      setTranslatedImage(imageUrl);
      message.success("翻译重绘成功！");
    } catch (e: any) {
      console.error(e);
      message.error(`翻译失败: ${e.message || "无法连接到服务器或认证令牌无效"}`);
    } finally {
      setIsTranslating(false);
    }
  };

  const functionList = [
    {
      title: "截图",
      description: "双击系统托盘图标，或通过快捷键开始划定截图框选区域。",
      icon: <CameraOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: config.hotkey || "Alt+A",
      disabled: false,
      buttonText: "立即截图",
      onClick: onStartScreenshot,
    },
    {
      title: "截图翻译",
      description: "按 Alt+T 直接进入翻译模式，框选后自动调用翻译接口并原位重绘译文。",
      icon: <TranslationOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: "Alt+T",
      disabled: false,
      buttonText: "截图翻译",
      onClick: async () => {
        try {
          await invoke("start_screenshot", { mode: "translate" });
        } catch (e: any) {
          message.error("启动截图翻译失败: " + e.toString());
        }
      },
    },
    {
      title: "延迟截图",
      description: delayedActive ? `倒计时 ${delayedCountdown} 秒后自动调出截图选区…` : "倒计时 3 秒后自动调出截图选区，用于捕获悬浮菜单或下拉状态。",
      icon: <ClockCircleOutlined style={{ fontSize: 18, color: delayedActive ? "#1677ff" : "#fa8c16" }} />,
      hotkey: "未设置",
      disabled: false,
      buttonText: delayedActive ? `⏱ ${delayedCountdown}s` : "延迟截图",
      onClick: handleDelayedScreenshot,
    },
    {
      title: "文本识别 (OCR)",
      description: "在截图框选后，点击悬浮工具栏中的「识字」按钮，识别文字并自动复制到剪贴板。",
      icon: <ScanOutlined style={{ fontSize: 18, color: "#722ed1" }} />,
      hotkey: "悬浮工具栏",
      disabled: false,
      buttonText: "需要先截图",
      onClick: onStartScreenshot,
    },
    {
      title: "文本识别翻译",
      description: "框选屏幕物理像素后，在原地重绘渲染替换为对应的译文。",
      icon: <TranslationOutlined style={{ fontSize: 18, color: "#eb2f96" }} />,
      hotkey: "Ctrl+Q",
      disabled: false,
      buttonText: "需要先截图",
      onClick: onStartScreenshot,
    },
    {
      title: "复制到剪贴板",
      description: "截图后自动将截图或翻译后的像素流以图片数据写入剪贴板。",
      icon: <CopyOutlined style={{ fontSize: 18, color: "#13c2c2" }} />,
      hotkey: "Enter",
      disabled: false,
      buttonText: "需要先截图",
      onClick: onStartScreenshot,
    },
    {
      title: "截取全屏",
      description: "快速截取主显示器的当前完整物理画面，直接复制到剪贴板。",
      icon: <DesktopOutlined style={{ fontSize: 18, color: "#2f54eb" }} />,
      hotkey: "未设置",
      disabled: false,
      buttonText: "截取全屏",
      onClick: handleFullscreenCapture,
    },
    {
      title: "当前活动窗口截图",
      description: "打开截图模式后，通过框选捕捉焦点窗口的内容（目前使用框选方式）。",
      icon: <BorderInnerOutlined style={{ fontSize: 18, color: "#a0d911" }} />,
      hotkey: "Alt+A → 框选",
      disabled: false,
      buttonText: "框选窗口",
      onClick: onStartScreenshot,
    },
  ];

  const tabItems = [
    {
      key: "screenshot",
      label: "截图功能",
      children: (
        <List
          itemLayout="horizontal"
          dataSource={functionList}
          renderItem={(item) => (
            <List.Item
              style={{
                background: "#ffffff",
                border: "1px solid #f0f0f0",
                borderRadius: 12,
                padding: "12px 20px",
                marginBottom: 10,
                height: 56,
              }}
              actions={[
                <Space size="middle" align="center" key="actions">
                  {item.title === "截图" && shortcutError ? (
                    <Tooltip title={`快捷键 Alt+A 注册失败: ${shortcutError}。可能是热键冲突，请重启以重新尝试。`}>
                      <Tag
                        color="error"
                        style={{
                          margin: 0, border: "1px dashed #ffa39e",
                          backgroundColor: "#fff2f0", color: "#ff4d4f",
                          fontWeight: "600", height: 22,
                          display: "inline-flex", alignItems: "center"
                        }}
                      >
                        {item.hotkey} (冲突)
                      </Tag>
                    </Tooltip>
                  ) : item.hotkey === "未设置" ? (
                    <Tag style={{ margin: 0, border: "1px solid #d9d9d9", color: "#bfbfbf", height: 22, display: "inline-flex", alignItems: "center" }}>
                      未设置
                    </Tag>
                  ) : (
                    <Tag
                      style={{
                        margin: 0, border: "1px solid #91d5ff",
                        backgroundColor: "#e6f7ff", color: "#1677ff",
                        fontWeight: "600", height: 22,
                        display: "inline-flex", alignItems: "center",
                        fontFamily: "Consolas, Monaco, monospace"
                      }}
                    >
                      {item.hotkey}
                    </Tag>
                  )}
                  {item.disabled ? (
                    <Tooltip title="开发中">
                      <span style={{ display: "inline-block", cursor: "not-allowed" }}>
                        <Button
                          type="default"
                          size="small"
                          disabled
                          style={{ height: 32, fontSize: 12, pointerEvents: "none" }}
                        >
                          {item.buttonText}
                        </Button>
                      </span>
                    </Tooltip>
                  ) : (
                    <Button
                      type="primary"
                      size="small"
                      onClick={item.onClick}
                      style={{ height: 32, fontSize: 12 }}
                    >
                      {item.buttonText}
                    </Button>
                  )}
                </Space>,
              ]}
            >
              <List.Item.Meta
                avatar={item.icon}
                title={
                  <Text strong style={{ color: item.disabled ? "#8c8c8c" : "#1f1f1f", fontSize: 13 }}>
                    {item.title}
                  </Text>
                }
                description={
                  <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: -2 }}>
                    {item.description}
                  </Text>
                }
              />
            </List.Item>
          )}
        />
      ),
    },
    {
      key: "translate",
      label: "接口测试",
      children: (
        <Space direction="vertical" size="large" style={{ width: "100%" }}>
          {/* Telemetry connection status details */}
          <Row gutter={16}>
            <Col span={8}>
              <Card bordered={false} style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
                <Flex justify="space-between" align="start" style={{ marginBottom: 12 }}>
                  <Text type="secondary" style={{ fontSize: 11, fontWeight: "600" }}>
                    服务器连接
                  </Text>
                  <WifiOutlined style={{ color: "#bfbfbf" }} />
                </Flex>
                {serverStatus === "online" && <Tag color="success">在线</Tag>}
                {serverStatus === "offline" && <Tag color="error">离线</Tag>}
                {serverStatus === "checking" && <Tag color="warning">检测中...</Tag>}
                <div style={{ marginTop: 8, fontSize: 10, color: "#8c8c8c" }}>
                  配置地址: {config.serverUrl || "未配置"}
                </div>
              </Card>
            </Col>

            <Col span={8}>
              <Card bordered={false} style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
                <Flex justify="space-between" align="start" style={{ marginBottom: 12 }}>
                  <Text type="secondary" style={{ fontSize: 11, fontWeight: "600" }}>
                    翻译通道
                  </Text>
                  <GlobalOutlined style={{ color: "#bfbfbf" }} />
                </Flex>
                <Text strong style={{ fontSize: 14 }}>
                  {config.channel === "baidu" ? "百度翻译" : config.channel === "new-api" ? "大模型翻译" : "谷歌翻译 (默认)"}
                </Text>
                <div style={{ marginTop: 8, fontSize: 10, color: "#8c8c8c" }}>
                  本地 OCR: {config.useLocalOcr ? "启用" : "禁用"}
                </div>
              </Card>
            </Col>

            <Col span={8}>
              <Card bordered={false} style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
                <Flex justify="space-between" align="start" style={{ marginBottom: 12 }}>
                  <Text type="secondary" style={{ fontSize: 11, fontWeight: "600" }}>
                    延迟响应
                  </Text>
                  <DashboardOutlined style={{ color: "#bfbfbf" }} />
                </Flex>
                <Text strong style={{ fontSize: 18 }}>
                  {serverStatus === "online" && responseTime ? `${responseTime} ms` : "—"}
                </Text>
                <div style={{ marginTop: 8, fontSize: 10, color: "#8c8c8c" }}>
                  平均响应阈值: ~350ms
                </div>
              </Card>
            </Col>
          </Row>

          {/* Tester module */}
          <Card bordered={false} style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
            <div style={{ marginBottom: 16 }}>
              <Text strong style={{ fontSize: 14 }}>
                连通性测试 (Drag-and-Drop Image Tester)
              </Text>
              <Paragraph type="secondary" style={{ fontSize: 12, margin: "4px 0 0 0" }}>
                拖拽一张包含英文文本的截图到下方，即可在网页内直观预览 OCR 识别及重绘嵌入后的中文图像。
              </Paragraph>
            </div>

            <Row gutter={24}>
              <Col span={12}>
                <Upload.Dragger
                  accept="image/*"
                  showUploadList={false}
                  customRequest={handleCustomUpload}
                  style={{
                    borderRadius: 12,
                    background: "#fafafa",
                    border: "2px dashed #d9d9d9",
                    padding: "24px 0",
                    height: 280,
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "center",
                  }}
                >
                  {previewUrl ? (
                    <Flex vertical align="center" justify="center" style={{ height: "100%" }}>
                      <img
                        src={previewUrl}
                        alt="Preview"
                        style={{ maxHeight: 150, maxWidth: "90%", objectFit: "contain", borderRadius: 8, marginBottom: 16 }}
                      />
                      <Space>
                        <Upload accept="image/*" showUploadList={false} customRequest={handleCustomUpload}>
                          <Button size="small">重新选择</Button>
                        </Upload>
                        <Button
                          type="primary"
                          size="small"
                          onClick={(e) => {
                            e.stopPropagation();
                            startTranslation();
                          }}
                          loading={isTranslating}
                          disabled={serverStatus !== "online"}
                        >
                          开始翻译
                        </Button>
                      </Space>
                    </Flex>
                  ) : (
                    <Flex vertical align="center" justify="center">
                      <p className="ant-upload-drag-icon" style={{ margin: 0, color: "#1677ff" }}>
                        <InboxOutlined style={{ fontSize: 40 }} />
                      </p>
                      <Text strong style={{ fontSize: 13, display: "block", marginTop: 12 }}>
                        拖拽图片文件到此处
                      </Text>
                      <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 4 }}>
                        或者点击浏览本地文件
                      </Text>
                    </Flex>
                  )}
                </Upload.Dragger>
              </Col>

              <Col span={12}>
                <div
                  style={{
                    border: "1px solid #f0f0f0",
                    borderRadius: 12,
                    padding: 16,
                    background: "#fafafa",
                    height: 280,
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "space-between",
                  }}
                >
                  <Flex justify="space-between" align="center" style={{ borderBottom: "1px solid #f0f0f0", paddingBottom: 10 }}>
                    <Text strong style={{ fontSize: 12 }}>
                      翻译结果预览
                    </Text>
                    {translatedImage && (
                      <Button
                        type="link"
                        size="small"
                        icon={<SaveOutlined />}
                        href={translatedImage}
                        download="translated.png"
                        style={{ fontSize: 11, padding: 0 }}
                      >
                        保存图片
                      </Button>
                    )}
                  </Flex>

                  <div style={{ flex: 1, display: "flex", alignItems: "center", justifyItems: "center", justifyContent: "center", padding: 12 }}>
                    {translatedImage ? (
                      <img
                        src={translatedImage}
                        alt="Result"
                        style={{ maxHeight: 150, maxWidth: "100%", objectFit: "contain", borderRadius: 8, boxShadow: "0 2px 8px rgba(0,0,0,0.06)" }}
                      />
                    ) : isTranslating ? (
                      <Flex vertical align="center" gap="small">
                        <SyncOutlined spin style={{ fontSize: 24, color: "#1677ff" }} />
                        <Text type="secondary" style={{ fontSize: 11 }}>
                          深度学习重绘排版中，约耗时 1s ...
                        </Text>
                      </Flex>
                    ) : (
                      <Empty description="暂无测试数据" image={Empty.PRESENTED_IMAGE_SIMPLE} />
                    )}
                  </div>

                  <Flex justify="space-between" style={{ borderTop: "1px solid #f0f0f0", paddingTop: 8, fontSize: 10, color: "#8c8c8c" }}>
                    <span>状态: {isTranslating ? "翻译中" : translatedImage ? "就绪" : "空闲"}</span>
                    <span>分辨率: —</span>
                  </Flex>
                </div>
              </Col>
            </Row>
          </Card>
        </Space>
      ),
    },
  ];

  return (
    <Card bordered={false} style={{ borderRadius: 12, boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
      <div style={{ marginBottom: 20 }}>
        <Title level={4} style={{ margin: 0 }}>
          控制面板
        </Title>
        <Paragraph type="secondary" style={{ fontSize: 12, margin: "4px 0 0 0" }}>
          管理快捷截图任务或对 N100 边缘节点的服务接口进行连通性测试。
        </Paragraph>
      </div>
      <Tabs defaultActiveKey="screenshot" items={tabItems} />
    </Card>
  );
}
