import React, { useState } from "react";
import { Alert, Button, Card, Col, Descriptions, Divider, List, Row, Select, Space, Tag, Typography, message } from "antd";
import {
  ApiOutlined,
  CheckCircleOutlined,
  CloudDownloadOutlined,
  FolderOpenOutlined,
  GlobalOutlined,
  LinkOutlined,
  ReloadOutlined,
  ToolOutlined,
} from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import useOcrConfigController from "../hooks/useOcrConfigController";
import useRapidOcrController from "../hooks/useRapidOcrController";
import type { RapidOcrModelInstallResult, RapidOcrModelVersion } from "../ocr-models";

const { Text, Title } = Typography;

const RAPIDOCR_DOCS_URL = "https://rapidai.github.io/RapidOCRDocs/main/model_list/";
const RAPIDOCR_MODELSCOPE_URL = "https://www.modelscope.cn/models/RapidAI/RapidOCR";
const RAPIDOCR_GITHUB_URL = "https://github.com/RapidAI/RapidOCR";

const modelOptions: Array<{ value: RapidOcrModelVersion; label: string }> = [
  { value: "v5", label: "Rapid OCR V5（默认）" },
  { value: "v4", label: "Rapid OCR V4（兼容）" },
];

function formatElapsed(ms?: number) {
  if (!ms) return "";
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}

export default function ModelManagement() {
  const { config, setConfig, saveConfig } = useOcrConfigController();
  const rapidOcr = useRapidOcrController({ autoRefresh: true });
  const [installing, setInstalling] = useState(false);
  const [installResult, setInstallResult] = useState<RapidOcrModelInstallResult | null>(null);

  const modelVersion = (config.rapidOcrModelVersion || "v5") as RapidOcrModelVersion;
  const modelRoot = installResult?.modelRoot || rapidOcr.status?.modelRoot || rapidOcr.status?.modelDir || "models\\rapidocr";
  const missingModels = rapidOcr.status?.missingModelFiles || [];
  const ready = Boolean(rapidOcr.status?.ready);
  const modelPackReady = Boolean(rapidOcr.status?.modelPacksReady);

  const saveRapidOcrModelVersion = async (rapidOcrModelVersion: RapidOcrModelVersion) => {
    setConfig({ ...config, rapidOcrModelVersion });
    await saveConfig({ rapidOcrModelVersion }, false);
    await rapidOcr.refreshStatus();
  };

  const openModelDir = async () => {
    await invoke("open_path_in_file_manager", { path: modelRoot });
  };

  const installModels = async () => {
    setInstalling(true);
    try {
      const result = await invoke<RapidOcrModelInstallResult>("install_rapid_ocr_models");
      setInstallResult(result);
      await rapidOcr.refreshStatus();
      if (result.ok) {
        message.success(`RapidOCR 模型已安装到 ${result.modelRoot}`);
      } else {
        message.warning("模型下载完成，但仍有文件需要检查。");
      }
    } catch (error: any) {
      message.error(`模型下载/安装失败：${error?.message || error}`);
    } finally {
      setInstalling(false);
    }
  };

  return (
    <Space direction="vertical" size={16} style={{ width: "100%" }}>
      <Card bordered={false} style={{ borderRadius: 20, background: "linear-gradient(135deg, #eef6ff 0%, #f8fbff 58%, #f5f3ff 100%)" }}>
        <Space direction="vertical" size={10} style={{ width: "100%" }}>
          <Space wrap>
            <Tag color="blue">RapidOCR</Tag>
            <Tag color="purple">Rapid OCR V5 / V4</Tag>
            <Tag color="green">ModelScope 官方源</Tag>
          </Space>
          <div>
            <Title level={4} style={{ margin: 0, color: "#0f172a" }}>模型管理</Title>
            <Text type="secondary" style={{ display: "block", marginTop: 6 }}>
              下载、安装和检查 RapidOCR 识字模型。默认安装到项目根目录下的 <Text code>models\rapidocr</Text>。
            </Text>
          </div>
        </Space>
      </Card>

      <Row gutter={[16, 16]} align="stretch">
        <Col xs={24} xl={14}>
          <Card
            bordered={false}
            title={<span><ApiOutlined style={{ marginRight: 8 }} />RapidOCR 模型包</span>}
            extra={<Button size="small" icon={<ReloadOutlined />} loading={rapidOcr.loadingStatus} onClick={rapidOcr.refreshStatus}>重新检测</Button>}
            style={{ height: "100%", borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}
          >
            <Space direction="vertical" size={14} style={{ width: "100%" }}>
              <Alert
                type={ready ? "success" : modelPackReady ? "info" : "warning"}
                showIcon
                message={ready ? "模型可用" : modelPackReady ? "模型文件已就绪，建议运行自测" : "模型缺失或待安装"}
                description={
                  ready
                    ? `当前 Rapid OCR ${modelVersion.toUpperCase()} 已可用于截图识字和截图翻译。`
                    : modelPackReady
                      ? "模型文件已经存在，点击自测确认 runner 和 ONNXRuntime 可以正常初始化。"
                      : `点击“一键下载/安装模型”，应用会从 RapidOCR 官方 ModelScope 模型源下载到 ${modelRoot}。`
                }
              />

              <Space wrap>
                <Button type="primary" icon={<CloudDownloadOutlined />} loading={installing} onClick={installModels}>
                  一键下载/安装模型
                </Button>
                <Button icon={<ToolOutlined />} loading={rapidOcr.selfTesting} onClick={rapidOcr.runSelfTest}>
                  运行模型自测
                </Button>
                <Button icon={<FolderOpenOutlined />} onClick={openModelDir}>
                  打开模型目录
                </Button>
                <Tag color={ready ? "green" : "orange"} icon={ready ? <CheckCircleOutlined /> : undefined}>
                  {ready ? "已就绪" : "待处理"}
                </Tag>
              </Space>

              <Space wrap align="center">
                <Text strong>当前模型版本</Text>
                <Select value={modelVersion} options={modelOptions} style={{ width: 220 }} onChange={(value) => saveRapidOcrModelVersion(value)} />
              </Space>

              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label="默认下载目录">{modelRoot}</Descriptions.Item>
                <Descriptions.Item label="当前模型">Rapid OCR {modelVersion.toUpperCase()}</Descriptions.Item>
                <Descriptions.Item label="缺失文件">{missingModels.length ? missingModels.join("、") : "无"}</Descriptions.Item>
                <Descriptions.Item label="最近安装耗时">{formatElapsed(installResult?.elapsedMs) || "未运行"}</Descriptions.Item>
              </Descriptions>
            </Space>
          </Card>
        </Col>

        <Col xs={24} xl={10}>
          <Card
            bordered={false}
            title={<span><GlobalOutlined style={{ marginRight: 8 }} />官方来源</span>}
            style={{ height: "100%", borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}
          >
            <Space direction="vertical" size={14} style={{ width: "100%" }}>
              <Alert
                type="info"
                showIcon
                message="下载源说明"
                description="RapidOCR v3 内置模型清单，模型托管在魔搭 ModelScope。应用的一键安装会调用 RapidOCR 官方清单下载，不走第三方未知链接。"
              />
              <Space direction="vertical" style={{ width: "100%" }}>
                <Button block icon={<LinkOutlined />} onClick={() => openUrl(RAPIDOCR_DOCS_URL)}>打开 RapidOCR 模型文档</Button>
                <Button block icon={<LinkOutlined />} onClick={() => openUrl(RAPIDOCR_MODELSCOPE_URL)}>打开 ModelScope 模型仓库</Button>
                <Button block icon={<LinkOutlined />} onClick={() => openUrl(RAPIDOCR_GITHUB_URL)}>打开 RapidOCR GitHub</Button>
              </Space>
              <Divider style={{ margin: "4px 0" }} />
              <List
                size="small"
                header={<Text strong>手动下载方式</Text>}
                dataSource={[
                  "打开 ModelScope 的 RapidAI/RapidOCR 模型仓库。",
                  "下载本应用需要的 ONNX 模型和字典文件。",
                  "把文件放到根目录 models\\rapidocr。",
                  "回到本页点击重新检测或运行模型自测。",
                ]}
                renderItem={(item, index) => <List.Item><Text>{index + 1}. {item}</Text></List.Item>}
              />
            </Space>
          </Card>
        </Col>
      </Row>
    </Space>
  );
}
