import { Alert, Button, Card, Descriptions, Select, Space, Switch, Tag, Typography } from "antd";
import {
  ApiOutlined,
  CheckCircleOutlined,
  ExperimentOutlined,
  FolderOpenOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  ThunderboltOutlined,
} from "@ant-design/icons";
import { invoke } from "@tauri-apps/api/core";
import type { RapidOcrModelVersion, RapidOcrSelfTestResult, RapidOcrStatus } from "../../ocr-models";

const { Text } = Typography;

type RapidOcrPanelProps = {
  status: RapidOcrStatus | null;
  loadingStatus: boolean;
  selfTesting: boolean;
  workerBusy: boolean;
  modelVersion: RapidOcrModelVersion;
  workerEnabled: boolean;
  lastSelfTest?: RapidOcrSelfTestResult | null;
  onModelVersionChange: (version: RapidOcrModelVersion) => Promise<void>;
  onWorkerEnabledChange: (enabled: boolean) => Promise<void>;
  onRefreshStatus: () => void;
  onRunSelfTest: () => void;
  onStartWorker: () => void;
  onStopWorker: () => void;
  onRestartWorker: () => void;
};

const modelOptions: Array<{ value: RapidOcrModelVersion; label: string }> = [
  { value: "v5", label: "Rapid OCR V5（默认）" },
  { value: "v4", label: "Rapid OCR V4（兼容）" },
];

function workerStateText(status: RapidOcrStatus | null, workerEnabled: boolean) {
  if (!workerEnabled) return { color: "default", text: "已关闭" };
  if (status?.workerRunning) return { color: "green", text: "运行中" };
  return { color: "orange", text: "待启动" };
}

export default function RapidOcrPanel({
  status,
  loadingStatus,
  selfTesting,
  workerBusy,
  modelVersion,
  workerEnabled,
  lastSelfTest,
  onModelVersionChange,
  onWorkerEnabledChange,
  onRefreshStatus,
  onRunSelfTest,
  onStartWorker,
  onStopWorker,
  onRestartWorker,
}: RapidOcrPanelProps) {
  const ready = Boolean(status?.ready);
  const modelDir = status?.modelDir || "models/rapidocr";
  const workerState = workerStateText(status, workerEnabled);
  const cachedEngines = status?.worker?.cachedEngines || [];
  const statusAlert = ready
    ? {
        type: "success" as const,
        message: "RapidOCR 文本识别已就绪",
        description: workerEnabled
          ? "常驻识别服务会复用已加载模型；首次识别或自测会预热中英文模型，小字区域会自动增强重试。"
          : "当前使用一次性 runner。关闭常驻服务会降低内存占用，但每次识别会重新启动 OCR。",
      }
    : {
        type: "warning" as const,
        message: "RapidOCR 文本识别需要检查",
        description:
          status?.lastError ||
          "请运行自测；如果失败，请确认内置 rapidocr-runner.exe 与 models/rapidocr 模型目录都在便携包内。",
      };

  return (
    <Card
      bordered={false}
      title={
        <span>
          <ApiOutlined style={{ marginRight: 8 }} />
          文本识别
        </span>
      }
      extra={
        <Button size="small" icon={<ReloadOutlined />} loading={loadingStatus} onClick={onRefreshStatus}>
          刷新
        </Button>
      }
      style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}
    >
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <Alert
          type={statusAlert.type}
          showIcon
          message={statusAlert.message}
          description={statusAlert.description}
          action={
            <Button type={ready ? "default" : "primary"} icon={<ExperimentOutlined />} loading={selfTesting} onClick={onRunSelfTest}>
              运行自测
            </Button>
          }
        />

        <Card size="small" bordered={false} style={{ borderRadius: 14, background: "rgba(37,99,235,0.06)" }}>
          <Space direction="vertical" size={10} style={{ width: "100%" }}>
            <Space wrap align="center" style={{ width: "100%", justifyContent: "space-between" }}>
              <Space wrap align="center">
                <ThunderboltOutlined style={{ color: "#2563eb" }} />
                <Text strong>常驻 OCR 加速</Text>
                <Switch checked={workerEnabled} onChange={onWorkerEnabledChange} />
                <Tag color={workerState.color}>{workerState.text}</Tag>
                {status?.worker?.pid && <Tag color="blue">PID {status.worker.pid}</Tag>}
              </Space>
              <Space wrap>
                <Button size="small" icon={<PlayCircleOutlined />} loading={workerBusy || selfTesting} disabled={!workerEnabled || Boolean(status?.workerRunning)} onClick={onStartWorker}>
                  启动/预热
                </Button>
                <Button size="small" icon={<ReloadOutlined />} loading={workerBusy} disabled={!workerEnabled} onClick={onRestartWorker}>
                  重启
                </Button>
                <Button size="small" icon={<PauseCircleOutlined />} loading={workerBusy} disabled={!status?.workerRunning} onClick={onStopWorker}>
                  停止
                </Button>
              </Space>
            </Space>
            <Text type="secondary" style={{ fontSize: 12 }}>
              推荐开启：模型按需懒加载并常驻复用，热识别延迟明显低于每次启动 runner；关闭后自动回退一次性 runner。
            </Text>
          </Space>
        </Card>

        <Space wrap align="center">
          <Text strong>文本识别模型</Text>
          <Select
            value={modelVersion}
            options={modelOptions}
            style={{ width: 220 }}
            onChange={(value) => onModelVersionChange(value)}
          />
          <Tag color={ready ? "green" : "orange"} icon={ready ? <CheckCircleOutlined /> : undefined}>
            {ready ? "已就绪" : "待检查"}
          </Tag>
          <Tag color="blue">RapidOCR</Tag>
          <Tag color="cyan">源语言自动</Tag>
          <Tag color="purple">小字增强</Tag>
        </Space>

        <Descriptions size="small" column={2}>
          <Descriptions.Item label="主路径">RapidOCR / ONNXRuntime</Descriptions.Item>
          <Descriptions.Item label="当前模型">{modelVersion === "v5" ? "Rapid OCR V5" : "Rapid OCR V4"}</Descriptions.Item>
          <Descriptions.Item label="Runner">{status?.runnerKind || "检测中"}</Descriptions.Item>
          <Descriptions.Item label="模型目录">{modelDir}</Descriptions.Item>
          <Descriptions.Item label="Probe/预热耗时">
            {typeof status?.probeTimings?.total_ms === "number" ? `${status.probeTimings.total_ms}ms` : "未检测"}
          </Descriptions.Item>
          <Descriptions.Item label="最近自测">
            {lastSelfTest ? (lastSelfTest.ok ? "通过" : lastSelfTest.message) : "未运行"}
          </Descriptions.Item>
          <Descriptions.Item label="已缓存模型">
            {cachedEngines.length > 0
              ? cachedEngines.map((engine) => `${engine.version || modelVersion}:${engine.lang || "auto"}`).join("、")
              : workerEnabled
                ? "启动后显示"
                : "常驻关闭"}
          </Descriptions.Item>
          <Descriptions.Item label="服务错误">{status?.worker?.lastError || "无"}</Descriptions.Item>
        </Descriptions>

        <Space wrap>
          <Button size="small" icon={<FolderOpenOutlined />} onClick={() => invoke("open_path_in_file_manager", { path: modelDir })}>
            打开模型目录
          </Button>
          {status?.runnerPath && (
            <Text type="secondary" style={{ fontSize: 12 }}>
              Runner: {status.runnerPath}
            </Text>
          )}
        </Space>
      </Space>
    </Card>
  );
}
