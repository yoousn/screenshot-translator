import { Alert, Button, Card, Col, Descriptions, Row, Select, Space, Tag, Typography } from "antd";
import { ApiOutlined, CheckCircleOutlined, FolderOpenOutlined, ReloadOutlined, ToolOutlined } from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
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
  { value: "v5", label: "Rapid OCR V5（默认）" },
  { value: "v4", label: "Rapid OCR V4（兼容）" },
];

function StatusTile({ title, ready, detail }: { title: string; ready: boolean; detail: string }) {
  return (
    <Card size="small" bordered={false} style={{ height: "100%", borderRadius: 16, background: "#f8fafc" }}>
      <Space direction="vertical" size={6} style={{ width: "100%" }}>
        <Space align="center" style={{ width: "100%", justifyContent: "space-between" }}>
          <Text strong>{title}</Text>
          <Tag color={ready ? "green" : "orange"} style={{ margin: 0 }}>{ready ? "可用" : "缺失/待检测"}</Tag>
        </Space>
        <Text type="secondary" style={{ fontSize: 12 }}>{detail}</Text>
      </Space>
    </Card>
  );
}

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
  const modelDir = status?.modelDir || "models\\rapidocr";
  const missingModels = status?.missingModelFiles || [];
  const hasMissingModels = missingModels.length > 0;
  const alertType = ready ? "success" : hasMissingModels ? "warning" : "info";
  const alertMessage = ready ? "识字模型可用" : hasMissingModels ? "识字模型文件缺失" : "识字模型需要检测";
  const alertDescription = ready
    ? `当前使用 Rapid OCR ${(status?.rapidOcrModelVersion || modelVersion).toUpperCase()}，截图识字和截图翻译可用。`
    : hasMissingModels
      ? `模型目录缺少 ${missingModels.length} 个文件。请把 RapidOCR 模型放到下方目录后重新检测。`
      : status?.lastError || "点击重新检测或运行自测，确认 RapidOCR runner 和模型是否可用。";

  const openModelDir = () => {
    invoke("open_path_in_file_manager", { path: modelDir }).catch(() => undefined);
  };

  return (
    <Card
      bordered={false}
      title={<span><ApiOutlined style={{ marginRight: 8 }} />识字模型</span>}
      extra={<Button size="small" icon={<ReloadOutlined />} loading={loadingStatus} onClick={onRefreshStatus}>重新检测</Button>}
      style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}
    >
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <Alert
          type={alertType}
          showIcon
          message={alertMessage}
          description={alertDescription}
          action={
            <Button type={ready ? "default" : "primary"} icon={<ToolOutlined />} loading={selfTesting} onClick={onRunSelfTest}>
              运行自测
            </Button>
          }
        />

        <Row gutter={[12, 12]}>
          <Col xs={24} md={8}>
            <StatusTile
              title="RapidOCR runner"
              ready={Boolean(status?.runnerReady)}
              detail={status?.runnerPath ? status.runnerKind || "已找到 runner" : "未找到 runner 时无法执行本地识字。"}
            />
          </Col>
          <Col xs={24} md={8}>
            <StatusTile
              title="Rapid OCR V5 / V4"
              ready={Boolean(status?.modelPacksReady)}
              detail={hasMissingModels ? `缺少 ${missingModels.length} 个文件` : `当前模型：Rapid OCR ${modelVersion.toUpperCase()}`}
            />
          </Col>
          <Col xs={24} md={8}>
            <StatusTile
              title="模型自测"
              ready={Boolean(status?.selfTestReady)}
              detail={lastSelfTest ? (lastSelfTest.ok ? "最近自测通过" : lastSelfTest.message) : "首次使用建议运行一次自测。"}
            />
          </Col>
        </Row>

        <Space wrap align="center">
          <Text strong>模型版本</Text>
          <Select value={modelVersion} options={modelOptions} style={{ width: 220 }} onChange={(value) => onModelVersionChange(value)} />
          <Tag color={ready ? "green" : "orange"} icon={ready ? <CheckCircleOutlined /> : undefined}>
            {ready ? "已就绪" : "待处理"}
          </Tag>
          <Button icon={<FolderOpenOutlined />} onClick={openModelDir}>打开模型目录</Button>
        </Space>

        <Descriptions size="small" column={1} bordered>
          <Descriptions.Item label="模型目录">{modelDir}</Descriptions.Item>
          <Descriptions.Item label="当前模型">Rapid OCR {modelVersion.toUpperCase()}</Descriptions.Item>
          <Descriptions.Item label="缺失文件">{missingModels.length ? missingModels.join("、") : "无"}</Descriptions.Item>
        </Descriptions>
      </Space>
    </Card>
  );
}
