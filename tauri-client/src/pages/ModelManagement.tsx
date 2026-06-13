import React, { useEffect, useState } from "react";
import { Alert, Button, Card, Col, Descriptions, Divider, List, Progress, Row, Select, Space, Tag, Typography, message } from "antd";
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
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import useOcrConfigController from "../hooks/useOcrConfigController";
import useRapidOcrController from "../hooks/useRapidOcrController";
import {
  localOcrModelName,
  localOcrModelOptions,
  type RapidOcrModelInstallProgress,
  type RapidOcrModelInstallResult,
  type RapidOcrModelVersion,
} from "../ocr-models";

const { Text, Title } = Typography;

const RAPIDOCR_DOCS_URL = "https://rapidai.github.io/RapidOCRDocs/main/model_list/";
const RAPIDOCR_MODELSCOPE_URL = "https://www.modelscope.cn/models/RapidAI/RapidOCR";
const RAPIDOCR_GITHUB_URL = "https://github.com/RapidAI/RapidOCR";
const PADDLEOCR_GITHUB_URL = "https://github.com/PaddlePaddle/PaddleOCR";

function formatElapsed(ms?: number) {
  if (!ms) return "";
  if (ms < 1000) return `${ms} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}

export default function ModelManagement() {
  const { config, setConfig, saveConfig } = useOcrConfigController();
  const rapidOcr = useRapidOcrController();
  const [installing, setInstalling] = useState(false);
  const [installResult, setInstallResult] = useState<RapidOcrModelInstallResult | null>(null);
  const [installProgress, setInstallProgress] = useState<RapidOcrModelInstallProgress | null>(null);

  const modelVersion = (config.rapidOcrModelVersion || "v6") as RapidOcrModelVersion;
  const modelRoot = rapidOcr.status?.modelRoot || rapidOcr.status?.modelDir || (modelVersion === "v6" ? "ocrv6" : "models\\rapidocr");
  const missingModels = rapidOcr.status?.missingModelFiles || [];
  const ready = Boolean(rapidOcr.status?.ready);
  const modelPackReady = Boolean(rapidOcr.status?.modelPacksReady);
  const manualDownloadSteps = modelVersion === "v6"
    ? [
        "从 PaddleOCR 官方来源获取 PP-OCRv6 Small 检测与识别 ONNX 模型及对应 YAML。",
        "保持模型文件名不变，放入项目根目录 ocrv6。",
        "回到本页点击重新检测；只有 CTC 契约 probe 通过后才会显示已就绪。",
      ]
    : [
        "打开 ModelScope 的 RapidAI/RapidOCR 模型仓库。",
        "下载本应用需要的 ONNX 模型；ONNX 字符表已内嵌，不需要另下字典文件。",
        "把文件放到根目录 models\\rapidocr。",
        "回到本页点击重新检测或运行模型自测。",
      ];

  useEffect(() => {
    const unlisten = listen<RapidOcrModelInstallProgress>("rapidocr-model-install-progress", (event) => {
      setInstallProgress(event.payload);
    });
    return () => {
      unlisten.then((dispose) => dispose()).catch(() => undefined);
    };
  }, []);

  const saveRapidOcrModelVersion = async (rapidOcrModelVersion: RapidOcrModelVersion) => {
    const previousModelVersion = modelVersion;
    setConfig({ ...config, rapidOcrModelVersion });
    const saved = await saveConfig({ rapidOcrModelVersion }, false);
    if (!saved) {
      setConfig({ ...config, rapidOcrModelVersion: previousModelVersion });
      return;
    }
    const status = await rapidOcr.refreshStatus();
    const activeModelVersion = status?.rapidOcrModelVersion || rapidOcrModelVersion;
    const activeModelName = localOcrModelName(activeModelVersion);
    if (status?.ready) {
      message.success(`${activeModelName} 已保存并立即生效，下一次截图识字/翻译会使用该模型。`);
    } else {
      message.warning(`${activeModelName} 已保存，下一次截图会使用该模型；当前模型状态仍需检查或自测。`);
    }
  };

  const openModelDir = async () => {
    await invoke("open_path_in_file_manager", { path: modelRoot });
  };

  const installModels = async () => {
    setInstalling(true);
    setInstallProgress({ phase: "准备安装", detail: "正在启动 RapidOCR 模型安装器。", percent: 0, status: "active" });
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
      setInstallProgress({ phase: "安装失败", detail: error?.message || String(error), percent: 100, status: "exception" });
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
            <Tag color="blue">本地 OCR 引擎</Tag>
            <Tag color="purple">PP-OCRv6 Small 默认</Tag>
            <Tag color="green">RapidOCR 内部兼容适配器</Tag>
          </Space>
          <div>
            <Title level={4} style={{ margin: 0, color: "#0f172a" }}>本地 OCR 引擎</Title>
            <Text type="secondary" style={{ display: "block", marginTop: 6 }}>
              当前模型严格手动选择，不会自动切换或回退。V6 验证完成前，RapidOCR 继续保留为 V5 / V4 备用适配器。
            </Text>
          </div>
        </Space>
      </Card>

      <Row gutter={[16, 16]} align="stretch">
        <Col xs={24} xl={14}>
          <Card
            bordered={false}
            title={<span><ApiOutlined style={{ marginRight: 8 }} />本地 OCR 模型</span>}
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
                    ? `当前主模型：${localOcrModelName(modelVersion)}（手动选择），已可用于截图识字和截图翻译。`
                    : modelPackReady
                      ? "模型文件已经存在，点击自测确认 runner、ONNXRuntime 和模型契约可以正常初始化。"
                      : modelVersion === "v6"
                        ? `当前 V6 模型文件缺失。请将 PP-OCRv6 Small 文件放入 ${modelRoot} 后重新检测。`
                        : `点击“安装备用 V5 / V4 模型”，应用会从 RapidOCR 官方 ModelScope 模型源下载。`
                }
              />

              <Space wrap>
                <Button type="primary" icon={<CloudDownloadOutlined />} loading={installing} onClick={installModels}>
                  {modelVersion === "v6" ? "安装备用 V5 / V4 模型" : "下载/安装 V5 / V4 模型"}
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

              {installProgress && (
                <Space direction="vertical" size={4} style={{ width: "100%" }}>
                  <Progress
                    percent={installProgress.percent}
                    status={installProgress.status}
                    format={() => `${installProgress.phase} ${installProgress.percent}%`}
                  />
                  {installProgress.detail && <Text type="secondary" style={{ fontSize: 12 }}>{installProgress.detail}</Text>}
                </Space>
              )}

              <Space wrap align="center">
                <Text strong>当前模型版本</Text>
                <Select value={modelVersion} options={localOcrModelOptions} style={{ width: 280 }} onChange={(value) => saveRapidOcrModelVersion(value)} />
              </Space>

              <Descriptions size="small" column={1} bordered>
                <Descriptions.Item label="当前模型目录">{modelRoot}</Descriptions.Item>
                <Descriptions.Item label="当前模型">{localOcrModelName(modelVersion)}（手动选择）</Descriptions.Item>
                <Descriptions.Item label="兼容适配器">RapidOCR 保留用于 V5 / V4 备用模型</Descriptions.Item>
                <Descriptions.Item label="缺失文件">{missingModels.length ? missingModels.join("、") : "无"}</Descriptions.Item>
                <Descriptions.Item label="最近安装耗时">{formatElapsed(installResult?.elapsedMs) || "未运行"}</Descriptions.Item>
                <Descriptions.Item label="字典文件说明">
                  V6 字符表来自识别 YAML；初始化必须通过 18708 字典 / 18710 类 / 隐式空格契约 probe。
                </Descriptions.Item>
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
                message="模型来源说明"
                description="PP-OCRv6 Small 使用 PaddleOCR 官方模型；备用 V5 / V4 继续使用 RapidOCR 官方 ModelScope 清单。第一阶段不自动下载 V6。"
              />
              <Space direction="vertical" style={{ width: "100%" }}>
                <Button block icon={<LinkOutlined />} onClick={() => openUrl(PADDLEOCR_GITHUB_URL)}>打开 PaddleOCR GitHub</Button>
                <Button block icon={<LinkOutlined />} onClick={() => openUrl(RAPIDOCR_DOCS_URL)}>打开 RapidOCR 模型文档</Button>
                <Button block icon={<LinkOutlined />} onClick={() => openUrl(RAPIDOCR_MODELSCOPE_URL)}>打开 ModelScope 模型仓库</Button>
                <Button block icon={<LinkOutlined />} onClick={() => openUrl(RAPIDOCR_GITHUB_URL)}>打开 RapidOCR GitHub</Button>
              </Space>
              <Divider style={{ margin: "4px 0" }} />
              <List
                size="small"
                header={<Text strong>手动下载方式</Text>}
                dataSource={manualDownloadSteps}
                renderItem={(item, index) => <List.Item><Text>{index + 1}. {item}</Text></List.Item>}
              />
            </Space>
          </Card>
        </Col>
      </Row>
    </Space>
  );
}
