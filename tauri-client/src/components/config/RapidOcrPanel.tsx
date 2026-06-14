import { Alert, Button, Card, Col, Descriptions, Row, Select, Space, Tag, Typography } from "antd";
import { ApiOutlined, CheckCircleOutlined, FolderOpenOutlined, ReloadOutlined, ToolOutlined } from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import {
  localOcrModelName,
  localOcrModelOptions,
  type RapidOcrModelVersion,
  type RapidOcrSelfTestResult,
  type RapidOcrStatus,
} from "../../ocr-models";

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

function StatusTile({ title, ready, detail }: { title: string; ready: boolean; detail: string }) {
  return (
    <Card size="small" variant="borderless" style={{ height: "100%", borderRadius: 16, background: "#f8fafc" }}>
      <Space orientation="vertical" size={6} style={{ width: "100%" }}>
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
  const modelDir = status?.modelDir || (modelVersion === "v6" ? "ocrv6" : "models\\rapidocr");
  const missingModels = status?.missingModelFiles || [];
  const hasMissingModels = missingModels.length > 0;
  const alertType = ready ? "success" : hasMissingModels ? "warning" : "info";
  const alertMessage = ready ? "识字可用" : hasMissingModels ? "模型文件缺失" : "需要初始化本地 OCR";
  const alertDescription = ready
    ? `当前主模型：${localOcrModelName(status?.rapidOcrModelVersion || modelVersion)}（手动选择）。截图识字和截图翻译可用。`
    : hasMissingModels
      ? `模型目录缺少 ${missingModels.length} 个文件。请把当前所选模型放到下方目录后重新检测。`
      : status?.lastError || "点击初始化并应用，确认本地 OCR runner、ONNXRuntime 和当前模型可以真实加载。";

  const openModelDir = () => {
    invoke("open_path_in_file_manager", { path: modelDir }).catch(() => undefined);
  };

  return (
    <Card
      variant="borderless"
      title={<span><ApiOutlined style={{ marginRight: 8 }} />本地 OCR 引擎</span>}
      extra={<Button size="small" icon={<ReloadOutlined />} loading={loadingStatus} onClick={onRefreshStatus}>重新检测</Button>}
      style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}
    >
      <Space orientation="vertical" size={14} style={{ width: "100%" }}>
        <Alert
          type={alertType}
          showIcon
          title={alertMessage}
          description={alertDescription}
          action={
            <Button type={ready ? "default" : "primary"} icon={<ToolOutlined />} loading={selfTesting} onClick={onRunSelfTest}>
              初始化并应用
            </Button>
          }
        />

        <Row gutter={[12, 12]}>
          <Col xs={24} md={8}>
            <StatusTile
              title="本地 OCR runner"
              ready={Boolean(status?.runnerReady)}
              detail={status?.runnerPath ? status.runnerKind || "已找到 runner" : "未找到 runner 时无法执行本地识字。"}
            />
          </Col>
          <Col xs={24} md={8}>
            <StatusTile
              title="当前主模型"
              ready={Boolean(status?.modelPacksReady)}
              detail={hasMissingModels ? `缺少 ${missingModels.length} 个文件` : `当前模型：${localOcrModelName(modelVersion)}`}
            />
          </Col>
          <Col xs={24} md={8}>
            <StatusTile
              title="模型初始化"
              ready={Boolean(status?.selfTestReady)}
              detail={lastSelfTest ? (lastSelfTest.ok ? "最近初始化通过" : lastSelfTest.message) : "首次使用或切换模型后建议初始化一次。"}
            />
          </Col>
        </Row>

        <Space wrap align="center">
          <Text strong>模型版本</Text>
          <Select value={modelVersion} options={localOcrModelOptions} style={{ width: 260 }} onChange={(value) => onModelVersionChange(value)} />
          <Tag color={ready ? "green" : "orange"} icon={ready ? <CheckCircleOutlined /> : undefined}>
            {ready ? "已就绪" : "待处理"}
          </Tag>
          <Button icon={<FolderOpenOutlined />} onClick={openModelDir}>打开模型目录</Button>
        </Space>

        <Descriptions size="small" column={1} bordered>
          <Descriptions.Item label="模型目录">{modelDir}</Descriptions.Item>
          <Descriptions.Item label="当前模型">{localOcrModelName(modelVersion)}（手动选择）</Descriptions.Item>
          <Descriptions.Item label="兼容适配器">RapidOCR 保留用于 V5 / V4 备用模型</Descriptions.Item>
          <Descriptions.Item label="缺失文件">{missingModels.length ? missingModels.join("、") : "无"}</Descriptions.Item>
        </Descriptions>
      </Space>
    </Card>
  );
}
