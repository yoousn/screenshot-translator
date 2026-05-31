import React from "react";
import {
  Form,
  Input,
  InputNumber,
  Select,
  Switch,
  Button,
  Space,
  Card,
  Typography,
  Row,
  Col
} from "antd";
import {
  SaveOutlined,
  SlidersOutlined,
  GlobalOutlined,
  AppstoreOutlined,
  SyncOutlined,
  KeyOutlined
} from "@ant-design/icons";
import useSettingsController from "../hooks/useSettingsController";

const { Title, Paragraph, Text } = Typography;

interface SettingsProps {
  onConfigSaved: () => void;
}

export default function Settings({ onConfigSaved }: SettingsProps) {
  const [form] = Form.useForm();
  const {
    isSaving,
    isTestingBaidu,
    isTestingNewApi,
    isFetchingModels,
    availableModels,
    currentChannel,
    handleFormChange,
    fetchModels,
    testChannel,
    onFinish,
    restoreDefaultHotkeys,
  } = useSettingsController(form, onConfigSaved);


  const channelOptions = [
    { value: "google", label: "谷歌翻译 (默认/免密)" },
    { value: "baidu", label: "百度翻译 (开放平台)" },
    { value: "new-api", label: "中转大模型 (New API)" },
  ];

  const targetLangOptions = [
    { value: "zh", label: "中文" },
    { value: "en", label: "英语" },
    { value: "ja", label: "日语" },
    { value: "ko", label: "韩语" },
    { value: "fr", label: "法语" },
    { value: "de", label: "德语" },
    { value: "es", label: "西班牙语" },
  ];

  return (
    <Form
      form={form}
      layout="vertical"
      initialValues={{
        enableUiControlDetection: false,
        enableVisualDetection: false,
        detectionBorderWidth: 2,
        visualDetectionSensitivity: 3,
        useLocalOcr: true,
        fallbackToRemoteOcr: false,
        localOcrTimeoutMs: 5000,
        hotkey: "Alt+A",
        translateHotkey: "Alt+T",
      }}
      onFinish={onFinish}
      onValuesChange={handleFormChange}
      requiredMark={false}
      style={{ maxWidth: 800, margin: "0 auto" }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid #e8e8e8", paddingBottom: 16, marginBottom: 24 }}>
        <div>
          <Title level={4} style={{ margin: 0 }}>
            系统设置
          </Title>
          <Paragraph type="secondary" style={{ fontSize: 12, margin: "4px 0 0 0" }}>
            定制屏幕翻译系统的后端服务、翻译信道以及热键环境。
          </Paragraph>
        </div>
        <Button
          type="primary"
          icon={<SaveOutlined />}
          htmlType="submit"
          loading={isSaving}
          style={{ height: 36 }}
        >
          保存设置
        </Button>
      </div>

      <Space direction="vertical" size="middle" style={{ width: "100%" }}>
        <Card title={<span><SlidersOutlined style={{ marginRight: 8 }} />1. 后端翻译服务配置 (N100 Core)</span>} bordered={false}>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item
                label={<Text strong style={{ fontSize: 12 }}>API 服务器地址</Text>}
                name="serverUrl"
                rules={[{ required: true, message: "请输入 API 服务器地址" }]}
              >
                <Input placeholder="https://ocr.yousn.me" style={{ height: 32 }} />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -10 }}>
                部署在家庭私有云 (如 N100) 上的文本翻译服务接入端口。
              </Text>
            </Col>
            <Col span={12}>
              <Form.Item
                label={<Text strong style={{ fontSize: 12 }}>客户端认证令牌 (Token)</Text>}
                name="clientToken"
                rules={[{ required: true, message: "请输入客户端令牌" }]}
              >
                <Input.Password placeholder="请输入您的私有 client_token" prefix={<KeyOutlined />} style={{ height: 32 }} />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -10 }}>
                与服务器端 `client_token` 保持一致，避免未授权访问。
              </Text>
            </Col>
          </Row>
        </Card>

        <Card title={<span><GlobalOutlined style={{ marginRight: 8 }} />2. 翻译信道配置 (Translation Channels)</span>} bordered={false}>
          <Form.Item
            label={<Text strong style={{ fontSize: 12 }}>活动翻译信道</Text>}
            name="channel"
            initialValue="google"
          >
            <Select options={channelOptions} style={{ height: 32 }} />
          </Form.Item>

          <Form.Item
            label={<Text strong style={{ fontSize: 12 }}>目标语言</Text>}
            name="targetLang"
            initialValue="zh"
          >
            <Select options={targetLangOptions} style={{ height: 32 }} />
          </Form.Item>

          {currentChannel === "baidu" && (
            <Card type="inner" title="百度翻译参数" style={{ marginTop: 12 }}>
              <Row gutter={16}>
                <Col span={12}>
                  <Form.Item label="App ID" name="baiduAppId">
                    <Input placeholder="例如: 2026011900..." style={{ height: 32 }} />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="密匙 (Secret Key)" name="baiduSecretKey">
                    <Input.Password placeholder="密匙" style={{ height: 32 }} />
                  </Form.Item>
                </Col>
              </Row>
              <Button
                type="dashed"
                onClick={() => testChannel("baidu")}
                loading={isTestingBaidu}
                block
                style={{ height: 32 }}
              >
                测试连接并启用
              </Button>
            </Card>
          )}

          {currentChannel === "new-api" && (
            <Card type="inner" title="中转大模型配置" style={{ marginTop: 12 }}>
              <Row gutter={16}>
                <Col span={12}>
                  <Form.Item label="中转服务地址" name="newApiBase">
                    <Input placeholder="api.yousn.me" style={{ height: 32 }} />
                  </Form.Item>
                </Col>
                <Col span={12}>
                  <Form.Item label="API Key" name="newApiKey">
                    <Input.Password placeholder="sk-..." style={{ height: 32 }} />
                  </Form.Item>
                </Col>
              </Row>
              <Form.Item label="指定大语言模型 (Model)">
                <Space style={{ width: "100%" }}>
                  <Form.Item name="newApiModel" noStyle>
                    {availableModels.length > 0 ? (
                      <Select
                        options={availableModels.map((m) => ({ value: m, label: m }))}
                        style={{ height: 32, width: 280 }}
                      />
                    ) : (
                      <Input placeholder="gemini-3.5-flash" style={{ height: 32, width: 280 }} />
                    )}
                  </Form.Item>
                  <Button
                    icon={<SyncOutlined spin={isFetchingModels} />}
                    onClick={fetchModels}
                    style={{ height: 32 }}
                  >
                    拉取模型
                  </Button>
                </Space>
              </Form.Item>
              <Button
                type="dashed"
                onClick={() => testChannel("new-api")}
                loading={isTestingNewApi}
                block
                style={{ height: 32 }}
              >
                测试连接并启用
              </Button>
            </Card>
          )}
        </Card>

        <Card title="截图识别" bordered={false}>
          <Row gutter={24} style={{ marginBottom: 16 }}>
            <Col span={12}>
              <Form.Item label="启用 UI 控件识别" name="enableUiControlDetection" valuePropName="checked">
                <Switch />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                加入轻量 Win32 子控件候选；关闭后只识别窗口，速度最快。
              </Text>
            </Col>
            <Col span={12}>
              <Form.Item label="启用视觉区域识别" name="enableVisualDetection" valuePropName="checked">
                <Switch />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                通过图像边界补充识别不暴露窗口/控件的应用；如果误选明显，建议关闭。
              </Text>
            </Col>
          </Row>
          <Row gutter={24}>
            <Col span={12}>
              <Form.Item label="识别边框粗细 (px)" name="detectionBorderWidth">
                <InputNumber min={1} max={6} placeholder="2" style={{ width: "100%", height: 32 }} />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                控制悬停和选区的蓝色边框，推荐 1-2px。
              </Text>
            </Col>
            <Col span={12}>
              <Form.Item label="视觉识别灵敏度" name="visualDetectionSensitivity">
                <InputNumber min={1} max={5} placeholder="3" style={{ width: "100%", height: 32 }} />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                数值越高识别越积极，但可能更容易误选；推荐 2-3。
              </Text>
            </Col>
          </Row>
        </Card>

        <Card title={<span><AppstoreOutlined style={{ marginRight: 8 }} />4. 系统控制与热键</span>} bordered={false}>
          <Row gutter={24}>
            <Col span={12}>
              <Form.Item label="开机自动启动" name="autostart" valuePropName="checked">
                <Switch />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                在 Windows 启动时自动静默加载该系统托盘服务。
              </Text>
            </Col>
            <Col span={12}>
              <Form.Item
                label="全局截图快捷键"
                name="hotkey"
                rules={[{ pattern: /^(|((Alt|Ctrl|Control|Shift|Cmd|Command|Meta|Win|Windows)(\s*\+\s*(Alt|Ctrl|Control|Shift|Cmd|Command|Meta|Win|Windows))*\s*\+\s*.+))$/i, message: "格式示例：Alt+A，留空表示取消" }]}
              >
                <Input placeholder="Alt+A；留空取消" style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
              </Form.Item>
              <Form.Item
                label="翻译截图快捷键"
                name="translateHotkey"
                rules={[{ pattern: /^(|((Alt|Ctrl|Control|Shift|Cmd|Command|Meta|Win|Windows)(\s*\+\s*(Alt|Ctrl|Control|Shift|Cmd|Command|Meta|Win|Windows))*\s*\+\s*.+))$/i, message: "格式示例：Alt+T，留空表示取消" }]}
              >
                <Input placeholder="Alt+T；留空取消" style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
              </Form.Item>
              <Space>
                <Button onClick={() => form.setFieldsValue({ hotkey: "" })}>取消截图键</Button>
                <Button onClick={() => form.setFieldsValue({ translateHotkey: "" })}>取消翻译键</Button>
                <Button onClick={restoreDefaultHotkeys}>还原默认</Button>
              </Space>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: 8 }}>
                保存后立即生效；留空表示取消对应全局快捷键。
              </Text>
            </Col>
          </Row>
        </Card>
      </Space>
    </Form>
  );
}
