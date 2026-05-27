import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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
  Col,
  message
} from "antd";
import {
  SaveOutlined,
  SlidersOutlined,
  GlobalOutlined,
  FileTextOutlined,
  AppstoreOutlined,
  SyncOutlined,
  KeyOutlined
} from "@ant-design/icons";

const { Title, Paragraph, Text } = Typography;

interface SettingsProps {
  onConfigSaved: () => void;
}

export default function Settings({ onConfigSaved }: SettingsProps) {
  const [form] = Form.useForm();
  const [isSaving, setIsSaving] = useState(false);
  const [isTestingBaidu, setIsTestingBaidu] = useState(false);
  const [isTestingNewApi, setIsTestingNewApi] = useState(false);
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [currentChannel, setCurrentChannel] = useState<string>("google");

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(configStr);

      form.setFieldsValue(parsedConfig);
      if (parsedConfig.channel) {
        setCurrentChannel(parsedConfig.channel);
      }

      const autostartEnabled = await invoke<boolean>("is_autostart_enabled");
      form.setFieldValue("autostart", autostartEnabled);

      if (parsedConfig.newApiBase && parsedConfig.newApiKey) {
        setAvailableModels([parsedConfig.newApiModel || "gemini-3.5-flash"]);
      }
    } catch (error) {
      console.error(error);
      message.error("加载配置文件失败");
    }
  };

  const handleFormChange = (changedValues: any) => {
    if (changedValues.channel) {
      setCurrentChannel(changedValues.channel);
    }
  };

  const fetchModels = async () => {
    const serverUrl = form.getFieldValue("serverUrl");
    const clientToken = form.getFieldValue("clientToken") || "";
    const newApiBase = form.getFieldValue("newApiBase");
    const newApiKey = form.getFieldValue("newApiKey");

    if (!serverUrl) {
      message.error("请先配置并保存服务器 URL");
      return;
    }
    if (!newApiBase || !newApiKey) {
      message.error("请先填写中转地址和 API Key");
      return;
    }

    setIsFetchingModels(true);
    try {
      const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/config/fetch_models`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": clientToken,
        },
        body: JSON.stringify({
          base_url: newApiBase,
          api_key: newApiKey,
        }),
      });

      const resData = await response.json();
      if (resData.status === "success" && resData.models) {
        setAvailableModels(resData.models);
        message.success(`拉取模型成功，共获取 ${resData.models.length} 个模型`);
        if (resData.models.length > 0 && !resData.models.includes(form.getFieldValue("newApiModel"))) {
          form.setFieldValue("newApiModel", resData.models[0]);
        }
      } else {
        throw new Error(resData.error || "拉取失败");
      }
    } catch (e: any) {
      message.error(`获取模型列表失败: ${e.message}`);
    } finally {
      setIsFetchingModels(false);
    }
  };

  const testChannel = async (channel: "baidu" | "new-api") => {
    const serverUrl = form.getFieldValue("serverUrl");
    const clientToken = form.getFieldValue("clientToken") || "";

    if (!serverUrl) {
      message.error("请先配置服务器 URL");
      return;
    }

    const testPayload: any = {
      channel,
      config: {},
    };

    if (channel === "baidu") {
      setIsTestingBaidu(true);
      testPayload.config = {
        app_id: form.getFieldValue("baiduAppId"),
        secret_key: form.getFieldValue("baiduSecretKey"),
      };
    } else {
      setIsTestingNewApi(true);
      testPayload.config = {
        base_url: form.getFieldValue("newApiBase"),
        api_key: form.getFieldValue("newApiKey"),
        model: form.getFieldValue("newApiModel"),
      };
    }

    try {
      const response = await fetch(`${serverUrl.replace(/\/$/, "")}/api/config/test`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": clientToken,
        },
        body: JSON.stringify(testPayload),
      });

      const resData = await response.json();
      if (resData.status === "success") {
        message.success(`翻译通道 [${channel === "baidu" ? "百度" : "大模型"}] 测试通过，并已设为当前激活通道！`);
        form.setFieldValue("channel", channel);
        setCurrentChannel(channel);
      } else {
        throw new Error(resData.error || "接口验证失败");
      }
    } catch (e: any) {
      message.error(`测试连通性失败: ${e.message}`);
    } finally {
      setIsTestingBaidu(false);
      setIsTestingNewApi(false);
    }
  };

  const onFinish = async (values: any) => {
    setIsSaving(true);
    try {
      const { autostart: autostartVal, ...configValues } = values;
      const configStr = JSON.stringify(configValues, null, 4);
      await invoke("save_config", { configStr });
      await invoke("set_autostart_enabled", { enabled: autostartVal });

      message.success("设置保存成功！");
      onConfigSaved();
    } catch (error: any) {
      message.error(`保存失败: ${error.message || error}`);
    } finally {
      setIsSaving(false);
    }
  };

  const channelOptions = [
    { value: "google", label: "谷歌翻译 (默认/免密)" },
    { value: "baidu", label: "百度翻译 (开放平台)" },
    { value: "new-api", label: "中转大模型 (New API)" },
  ];

  return (
    <Form
      form={form}
      layout="vertical"
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
            定制屏幕翻译系统的云端服务、翻译信道、本地 OCR 以及热键环境。
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
        <Card title={<span><SlidersOutlined style={{ marginRight: 8 }} />1. 后端服务配置 (N100 Core)</span>} bordered={false}>
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
                部署在家庭私有云 (如 N100) 上的主服务接入端口。
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

        <Card title={<span><FileTextOutlined style={{ marginRight: 8 }} />3. 本地 OCR (PaddleOCR-json)</span>} bordered={false}>
          <Row gutter={24} style={{ marginBottom: 16 }}>
            <Col span={12}>
              <Form.Item label="启用本地 OCR" name="useLocalOcr" valuePropName="checked">
                <Switch />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                优先在客户端运行 PaddleOCR 可执行程序，避免图像上传云端识别的延迟。
              </Text>
            </Col>
            <Col span={12}>
              <Form.Item label="自动回退到云端 OCR" name="fallbackToRemoteOcr" valuePropName="checked">
                <Switch />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                当本地 OCR 出错或遇到超时响应时，自动交由 N100 云端 PaddleOCR 渲染。
              </Text>
            </Col>
          </Row>
          <Row gutter={16}>
            <Col span={18}>
              <Form.Item label="PaddleOCR-json.exe 物理路径" name="localOcrExecutablePath">
                <Input placeholder="C:/Users/.../PaddleOCR-json.exe" style={{ height: 32 }} />
              </Form.Item>
            </Col>
            <Col span={6}>
              <Form.Item label="超时限制 (ms)" name="localOcrTimeoutMs">
                <InputNumber min={500} max={30000} style={{ width: "100%", height: 32 }} />
              </Form.Item>
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
              <Form.Item label="全局截图快捷键" name="hotkey">
                <Input placeholder="Alt+A" style={{ height: 32, fontFamily: "monospace", textAlign: "center" }} />
              </Form.Item>
              <Text type="secondary" style={{ fontSize: 10, display: "block", marginTop: -20 }}>
                用于全局唤醒截图的物理按键组合。
              </Text>
            </Col>
          </Row>
        </Card>
      </Space>
    </Form>
  );
}
