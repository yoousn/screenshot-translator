import { Alert, Button, Card, Descriptions, Select, Space, Tag, Typography } from "antd";
import { ApiOutlined, CheckCircleOutlined, ExperimentOutlined, FolderOpenOutlined, ReloadOutlined } from "@ant-design/icons";
import { openPath } from "@tauri-apps/plugin-opener";
import type { RapidOcrModelVersion, RapidOcrSelfTestResult, RapidOcrStatus } from "../../ocr-models";

const { Text } = Typography;

type RapidOcrPanelProps = {
  status: RapidOcrStatus | null;
  loadingStatus: boolean;
  selfTesting: boolean;
  modelVersion: RapidOcrModelVersion;
  lastSelfTest?: RapidOcrSelfTestResult | null;
  onModelVersionChange: (version: RapidOcrModelVersion) => Promise<void>;
  onRefreshStatus: () => void;
  onRunSelfTest: () => void;
};

const modelOptions: Array<{ value: RapidOcrModelVersion; label: string }> = [
  { value: "v5", label: "Rapid OCR V5" },
  { value: "v4", label: "Rapid OCR V4" },
];

export default function RapidOcrPanel({
  status,
  loadingStatus,
  selfTesting,
  modelVersion,
  lastSelfTest,
  onModelVersionChange,
  onRefreshStatus,
  onRunSelfTest,
}: RapidOcrPanelProps) {
  const ready = Boolean(status?.ready);
  const modelDir = status?.modelDir || "models/ocr";
  const statusAlert = ready
    ? {
        type: "success" as const,
        message: "RapidOCR 文本识别已就绪",
        description: "当前截图 OCR 会直接使用 RapidOCR 主路径，源语言自动识别，翻译和原位重绘继续走现有链路。",
      }
    : {
        type: "warning" as const,
        message: "RapidOCR 文本识别需要检查",
        description: status?.lastError || "请运行自测；如果失败，请确认本机已安装 rapidocr / onnxruntime，或后续发布包已包含 rapidocr-runner.exe。",
      };

  return (
    <Card
      bordered={false}
      title={<span><ApiOutlined style={{ marginRight: 8 }} />文本识别</span>}
      extra={<Button size="small" icon={<ReloadOutlined />} loading={loadingStatus} onClick={onRefreshStatus}>刷新</Button>}
      style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}
    >
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <Alert
          type={statusAlert.type}
          showIcon
          message={statusAlert.message}
          description={statusAlert.description}
          action={<Button type={ready ? "default" : "primary"} icon={<ExperimentOutlined />} loading={selfTesting} onClick={onRunSelfTest}>运行自测</Button>}
        />

        <Space wrap align="center">
          <Text strong>文本识别模型</Text>
          <Select
            value={modelVersion}
            options={modelOptions}
            style={{ width: 180 }}
            onChange={(value) => onModelVersionChange(value)}
          />
          <Tag color={ready ? "green" : "orange"} icon={ready ? <CheckCircleOutlined /> : undefined}>
            {ready ? "已安装" : "待检查"}
          </Tag>
          <Tag color="blue">RapidOCR</Tag>
          <Tag color="cyan">源语言自动</Tag>
        </Space>

        <Descriptions size="small" column={2}>
          <Descriptions.Item label="主路径">RapidOCR / ONNXRuntime</Descriptions.Item>
          <Descriptions.Item label="当前模型">{modelVersion === "v5" ? "Rapid OCR V5" : "Rapid OCR V4"}</Descriptions.Item>
          <Descriptions.Item label="Runner">{status?.runnerKind || "检测中"}</Descriptions.Item>
          <Descriptions.Item label="模型目录">{modelDir}</Descriptions.Item>
          <Descriptions.Item label="Probe 耗时">
            {typeof status?.probeTimings?.total_ms === "number" ? `${status.probeTimings.total_ms}ms` : "未检测"}
          </Descriptions.Item>
          <Descriptions.Item label="最近自测">
            {lastSelfTest ? (lastSelfTest.ok ? "通过" : lastSelfTest.message) : "未运行"}
          </Descriptions.Item>
        </Descriptions>

        <Space wrap>
          <Button size="small" icon={<FolderOpenOutlined />} onClick={() => openPath(modelDir)}>打开模型目录</Button>
          {status?.runnerPath && <Text type="secondary" style={{ fontSize: 12 }}>Runner: {status.runnerPath}</Text>}
        </Space>
      </Space>
    </Card>
  );
}
