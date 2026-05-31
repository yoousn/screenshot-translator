import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Button,
  Card,
  Col,
  Flex,
  List,
  message,
  Row,
  Space,
  Statistic,
  Tag,
  Typography,
} from "antd";
import {
  CameraOutlined,
  ClockCircleOutlined,
  CopyOutlined,
  DesktopOutlined,
  ScanOutlined,
  TranslationOutlined,
} from "@ant-design/icons";

const { Text, Title, Paragraph } = Typography;

const T = {
  title: "\u63a7\u5236\u9762\u677f",
  desc: "\u7ba1\u7406\u622a\u56fe\u3001\u672c\u5730 OCR \u548c\u7ffb\u8bd1\u6d41\u7a0b\u3002\u5f53\u524d OCR \u5f3a\u5236\u5728\u5ba2\u6237\u7aef\u672c\u5730\u6267\u884c\uff0c\u4e0d\u518d\u4e0a\u4f20\u56fe\u7247\u5230 N100 \u505a\u4e91\u7aef OCR\u3002",
  screenshot: "\u622a\u56fe",
  screenshotDesc: "\u70b9\u51fb\u6216\u901a\u8fc7\u5feb\u6377\u952e\u5f00\u59cb\u6846\u9009\u622a\u56fe\u3002",
  translate: "\u622a\u56fe\u7ffb\u8bd1",
  translateDesc: "\u6846\u9009\u540e\u5148\u5728\u672c\u673a OCR\uff0c\u518d\u53ea\u628a\u6587\u672c\u53d1\u7ed9 N100 \u7ffb\u8bd1\u3002",
  delayed: "\u5ef6\u65f6\u622a\u56fe",
  delayedDesc: "3 \u79d2\u540e\u5f00\u59cb\u622a\u56fe\uff0c\u9002\u5408\u6355\u83b7\u83dc\u5355\u3001\u4e0b\u62c9\u6846\u6216\u60ac\u505c\u72b6\u6001\u3002",
  ocr: "\u672c\u5730\u8bc6\u5b57 OCR",
  ocrDesc: "\u6846\u9009\u540e\u5728\u5ba2\u6237\u7aef\u672c\u5730\u8bc6\u522b\u6587\u5b57\uff0c\u7ed3\u679c\u81ea\u52a8\u590d\u5236\u5230\u526a\u8d34\u677f\u3002",
  fullscreen: "\u5168\u5c4f\u590d\u5236",
  fullscreenDesc: "\u5feb\u901f\u622a\u53d6\u5f53\u524d\u5c4f\u5e55\u5e76\u590d\u5236\u5230\u526a\u8d34\u677f\u3002",
  run: "\u5f00\u59cb",
  server: "\u7ffb\u8bd1\u670d\u52a1",
  online: "\u5728\u7ebf",
  offline: "\u79bb\u7ebf",
  checking: "\u68c0\u6d4b\u4e2d",
  hotkey: "\u622a\u56fe\u5feb\u6377\u952e",
  ocrMode: "OCR \u6a21\u5f0f",
  localOnly: "\u672c\u5730\u5f3a\u5236",
  targetLang: "\u76ee\u6807\u8bed\u8a00",
  startScreenshot: "\u5f00\u59cb\u622a\u56fe",
  startTranslateFailed: "\u542f\u52a8\u622a\u56fe\u7ffb\u8bd1\u5931\u8d25\uff1a",
  delayedInfo: "3 \u79d2\u540e\u5f00\u59cb\u622a\u56fe\uff0c\u8bf7\u51c6\u5907\u597d\u8981\u622a\u53d6\u7684\u5185\u5bb9\u3002",
  fullscreenCopied: "\u5168\u5c4f\u622a\u56fe\u5df2\u590d\u5236\u5230\u526a\u8d34\u677f\u3002",
  fullscreenFailed: "\u5168\u5c4f\u622a\u56fe\u5931\u8d25\uff1a",
};

interface Config {
  serverUrl?: string;
  clientToken?: string;
  channel?: string;
  targetLang?: string;
  hotkey?: string;
}

interface DashboardProps {
  onStartScreenshot: () => void;
  shortcutError?: string | null;
  serverStatus: "checking" | "online" | "offline";
  responseTime: number | null;
  onRefreshStatus: () => void;
}

export default function Dashboard({ onStartScreenshot, shortcutError, serverStatus, responseTime }: DashboardProps) {
  const [config, setConfig] = useState<Config>({});
  const [delayedCountdown, setDelayedCountdown] = useState<number | null>(null);
  const [delayedActive, setDelayedActive] = useState(false);

  useEffect(() => {
    loadConfig();
  }, []);

  useEffect(() => {
    if (!delayedActive || delayedCountdown === null) return;
    if (delayedCountdown <= 0) {
      setDelayedCountdown(null);
      setDelayedActive(false);
      onStartScreenshot();
      return;
    }
    const timer = window.setTimeout(() => setDelayedCountdown((prev) => (prev !== null ? prev - 1 : null)), 1000);
    return () => window.clearTimeout(timer);
  }, [delayedActive, delayedCountdown, onStartScreenshot]);

  const loadConfig = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      setConfig(JSON.parse(configStr || "{}"));
    } catch (error) {
      console.error("Failed to load config:", error);
    }
  };

  const startTranslateScreenshot = async () => {
    try {
      await invoke("start_screenshot", { mode: "translate" });
    } catch (error: any) {
      message.error(`${T.startTranslateFailed}${error?.message || error}`);
    }
  };

  const handleDelayedScreenshot = () => {
    setDelayedCountdown(3);
    setDelayedActive(true);
    message.info(T.delayedInfo);
  };

  const handleFullscreenCapture = async () => {
    try {
      await invoke("quick_fullscreen_capture");
      message.success(T.fullscreenCopied);
    } catch (error: any) {
      message.error(`${T.fullscreenFailed}${error?.message || error}`);
    }
  };

  const statusText = serverStatus === "online" ? T.online : serverStatus === "offline" ? T.offline : T.checking;
  const statusColor = serverStatus === "online" ? "green" : serverStatus === "offline" ? "red" : "orange";

  const functionList = [
    {
      title: T.screenshot,
      description: T.screenshotDesc,
      icon: <CameraOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: config.hotkey || "Alt+A",
      buttonText: T.startScreenshot,
      danger: Boolean(shortcutError),
      onClick: onStartScreenshot,
    },
    {
      title: T.translate,
      description: T.translateDesc,
      icon: <TranslationOutlined style={{ fontSize: 18, color: "#1677ff" }} />,
      hotkey: "Alt+T",
      buttonText: T.translate,
      onClick: startTranslateScreenshot,
    },
    {
      title: T.delayed,
      description: T.delayedDesc,
      icon: <ClockCircleOutlined style={{ fontSize: 18, color: delayedActive ? "#1677ff" : "#fa8c16" }} />,
      hotkey: delayedActive ? `${delayedCountdown}s` : "Timer",
      buttonText: delayedActive ? `${delayedCountdown}s` : T.delayed,
      onClick: handleDelayedScreenshot,
    },
    {
      title: T.ocr,
      description: T.ocrDesc,
      icon: <ScanOutlined style={{ fontSize: 18, color: "#722ed1" }} />,
      hotkey: "\u5de5\u5177\u680f",
      buttonText: T.screenshot,
      onClick: onStartScreenshot,
    },
    {
      title: T.fullscreen,
      description: T.fullscreenDesc,
      icon: <DesktopOutlined style={{ fontSize: 18, color: "#2f54eb" }} />,
      hotkey: "Instant",
      buttonText: T.fullscreen,
      onClick: handleFullscreenCapture,
    },
  ];

  return (
    <Space direction="vertical" size={16} style={{ width: "100%" }}>
      <Card bordered={false} style={{ borderRadius: 16 }}>
        <Flex justify="space-between" align="center" wrap="wrap" gap={16}>
          <div>
            <Title level={4} style={{ margin: 0 }}>{T.title}</Title>
            <Paragraph type="secondary" style={{ margin: "6px 0 0", maxWidth: 720 }}>{T.desc}</Paragraph>
          </div>
          <Button type="primary" icon={<CameraOutlined />} onClick={onStartScreenshot}>{T.startScreenshot}</Button>
        </Flex>
      </Card>

      <Row gutter={[16, 16]}>
        <Col span={6}><Card bordered={false}><Statistic title={T.hotkey} value={config.hotkey || "Alt+A"} /></Card></Col>
        <Col span={6}><Card bordered={false}><Statistic title={T.ocrMode} value={T.localOnly} /></Card></Col>
        <Col span={6}><Card bordered={false}><Statistic title={T.targetLang} value={(config.targetLang || "zh").toUpperCase()} /></Card></Col>
        <Col span={6}><Card bordered={false}><Statistic title={T.server} value={responseTime ? `${responseTime}ms` : statusText} suffix={<Tag color={statusColor}>{statusText}</Tag>} /></Card></Col>
      </Row>

      <Card title={T.title} bordered={false} style={{ borderRadius: 16 }}>
        <List
          itemLayout="horizontal"
          dataSource={functionList}
          renderItem={(item) => (
            <List.Item
              actions={[
                <Space key="actions">
                  <Tag color={item.danger ? "error" : "blue"}>{item.hotkey}</Tag>
                  <Button type="primary" onClick={item.onClick}>{item.buttonText || T.run}</Button>
                </Space>,
              ]}
            >
              <List.Item.Meta
                avatar={item.icon}
                title={<Text strong>{item.title}</Text>}
                description={<Text type="secondary">{item.description}</Text>}
              />
            </List.Item>
          )}
        />
      </Card>
    </Space>
  );
}
