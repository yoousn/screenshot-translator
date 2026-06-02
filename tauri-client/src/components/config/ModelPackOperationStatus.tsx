import React from "react";
import { Alert, Progress, Space, Tag, Typography } from "antd";
import type { OcrModelPackOperation, OcrModelPackOperationPhase } from "../../ocr-models";
import { useI18n } from "../../i18n";

const { Text } = Typography;

interface ModelPackOperationStatusProps {
  operation: OcrModelPackOperation | null;
}

export default function ModelPackOperationStatus({ operation }: ModelPackOperationStatusProps) {
  const { text } = useI18n();
  const labels = text.config;
  if (!operation) return null;

  const phaseLabel: Record<OcrModelPackOperationPhase, string> = {
    queued: labels.operationQueued,
    "resolving-index": labels.operationResolvingIndex,
    downloading: labels.operationDownloading,
    verifying: labels.operationVerifying,
    installing: labels.operationInstalling,
    "self-testing": labels.operationSelfTesting,
    completed: labels.operationCompleted,
    failed: labels.operationFailed,
  };

  const isFailed = operation.phase === "failed";
  const isDone = operation.phase === "completed";

  return (
    <Alert
      type={isFailed ? "error" : isDone ? "success" : "info"}
      showIcon
      message={<Space wrap><Text strong>{phaseLabel[operation.phase] || operation.phase}</Text><Tag>{operation.packId}</Tag>{operation.recoverable && <Tag color="blue">{labels.recoverable}</Tag>}</Space>}
      description={
        <Space direction="vertical" size={6} style={{ width: "100%" }}>
          <Text>{operation.message}</Text>
          {operation.nextAction && <Text type="secondary">{labels.nextAction}: {operation.nextAction}</Text>}
          <Progress size="small" percent={operation.percent} status={isFailed ? "exception" : isDone ? "success" : "active"} />
        </Space>
      }
    />
  );
}
