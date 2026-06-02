import { Alert, Card, List, Space, Tag, Typography } from "antd";
import { CheckCircleOutlined, ExclamationCircleOutlined } from "@ant-design/icons";
import { useI18n } from "../../i18n";
import type { YsnOcrRuntimeStatus } from "../../ocr-models";
import type { RecordingInfo } from "./types";

const { Text } = Typography;

type ConfigRecoveryChecklistProps = {
  runtimeStatus: YsnOcrRuntimeStatus | null;
  recordingInfo: RecordingInfo | null;
};

type RecoveryItem = {
  key: string;
  title: string;
  detail: string;
  ready: boolean;
};

export default function ConfigRecoveryChecklist({ runtimeStatus, recordingInfo }: ConfigRecoveryChecklistProps) {
  const { text } = useI18n();
  const labels = text.config;
  const sourceReadiness = runtimeStatus?.sourceReadiness;
  const pendingModelIds = sourceReadiness?.pendingModelIds || [];

  const items: RecoveryItem[] = [
    {
      key: "trusted-sources",
      title: labels.recoveryTrustedSourcesTitle,
      detail: sourceReadiness?.ready
        ? labels.recoveryTrustedSourcesReady
        : `${labels.trustedSourcesPendingDesc} ${pendingModelIds.length ? `${labels.trustedSourcesPendingModels}: ${pendingModelIds.slice(0, 6).join(", ")}` : ""}`.trim(),
      ready: Boolean(sourceReadiness?.ready),
    },
    {
      key: "model-packs",
      title: labels.recoveryModelPacksTitle,
      detail: runtimeStatus?.modelPacksReady ? labels.recoveryModelPacksReady : labels.overviewModelPacksPending,
      ready: Boolean(runtimeStatus?.modelPacksReady),
    },
    {
      key: "ocr-self-test",
      title: labels.recoveryOcrSelfTestTitle,
      detail: runtimeStatus?.runtimeInferenceReady ? labels.overviewOcrRuntimeReady : labels.overviewOcrRuntimePending,
      ready: Boolean(runtimeStatus?.runtimeInferenceReady),
    },
    {
      key: "ffmpeg",
      title: labels.recoveryFfmpegTitle,
      detail: recordingInfo?.ffmpegFound ? labels.overviewRecordingReady : labels.overviewRecordingPending,
      ready: Boolean(recordingInfo?.ffmpegFound),
    },
    {
      key: "audio-devices",
      title: labels.recoveryAudioDevicesTitle,
      detail: recordingInfo?.audioDevices?.length ? labels.audioDevicesDetected.replace("{count}", String(recordingInfo.audioDevices.length)) : labels.recoveryAudioDevicesPending,
      ready: Boolean(recordingInfo?.audioDevices?.length),
    },
  ];

  const blockers = items.filter((item) => !item.ready);

  return (
    <Card size="small" bordered={false} style={{ borderRadius: 16, background: "rgba(255,255,255,0.76)", boxShadow: "0 10px 30px rgba(15,23,42,0.05)" }}>
      <Space direction="vertical" size={10} style={{ width: "100%" }}>
        <Space align="center" style={{ width: "100%", justifyContent: "space-between" }}>
          <div>
            <Text strong>{labels.recoveryChecklistTitle}</Text>
            <Text type="secondary" style={{ display: "block", fontSize: 12 }}>{labels.recoveryChecklistDesc}</Text>
          </div>
          <Tag color={blockers.length ? "orange" : "green"}>{blockers.length ? labels.recoveryChecklistPending.replace("{count}", String(blockers.length)) : labels.recoveryChecklistReady}</Tag>
        </Space>
        {blockers.length > 0 && <Alert type="warning" showIcon message={labels.recoveryChecklistWarning} />}
        <List
          size="small"
          dataSource={items}
          renderItem={(item) => (
            <List.Item>
              <List.Item.Meta
                avatar={item.ready ? <CheckCircleOutlined style={{ color: "#16a34a" }} /> : <ExclamationCircleOutlined style={{ color: "#f97316" }} />}
                title={<Space wrap><Text strong>{item.title}</Text><Tag color={item.ready ? "green" : "orange"}>{item.ready ? labels.readinessReady : labels.readinessAction}</Tag></Space>}
                description={<Text type="secondary">{item.detail}</Text>}
              />
            </List.Item>
          )}
        />
      </Space>
    </Card>
  );
}
