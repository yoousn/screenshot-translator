import { Card, List, Space, Tag, Typography } from "antd";
import { CheckCircleOutlined, ClockCircleOutlined, ExclamationCircleOutlined } from "@ant-design/icons";
import type { OcrRuntimeReadinessStep } from "../../ocr-models";
import { localizeOcrReadinessStep } from "./ocrReadinessStepText";

const { Text } = Typography;

type OcrRuntimeReadinessStepsProps = {
  steps?: OcrRuntimeReadinessStep[];
  labels: Record<string, string>;
};

const stepColor = (step: OcrRuntimeReadinessStep) => {
  if (step.ready || step.severity === "success") return "green";
  if (step.severity === "error") return "red";
  return "orange";
};

const stepIcon = (step: OcrRuntimeReadinessStep) => {
  if (step.ready) return <CheckCircleOutlined style={{ color: "#16a34a" }} />;
  if (step.severity === "error") return <ExclamationCircleOutlined style={{ color: "#dc2626" }} />;
  return <ClockCircleOutlined style={{ color: "#f97316" }} />;
};

export default function OcrRuntimeReadinessSteps({ steps, labels }: OcrRuntimeReadinessStepsProps) {
  if (!steps?.length) return null;
  const displaySteps = steps.map((step) => localizeOcrReadinessStep(step, labels));
  const readyCount = displaySteps.filter((step) => step.ready).length;

  return (
    <Card size="small" bordered={false} style={{ borderRadius: 16, background: "rgba(248,250,252,0.92)" }}>
      <Space direction="vertical" size={10} style={{ width: "100%" }}>
        <Space align="center" style={{ width: "100%", justifyContent: "space-between" }}>
          <div>
            <Text strong>{labels.runtimeReadinessStepsTitle}</Text>
            <Text type="secondary" style={{ display: "block", fontSize: 12 }}>{labels.runtimeReadinessStepsDesc}</Text>
          </div>
          <Tag color={readyCount === displaySteps.length ? "green" : "orange"}>{readyCount}/{displaySteps.length} {labels.readinessReady}</Tag>
        </Space>
        <List
          size="small"
          dataSource={displaySteps}
          renderItem={(step) => (
            <List.Item>
              <List.Item.Meta
                avatar={stepIcon(step)}
                title={<Space wrap><Text strong>{step.displayLabel}</Text><Tag color={stepColor(step)}>{step.ready ? labels.readinessReady : labels.readinessAction}</Tag><Tag>{step.displayNextAction}</Tag></Space>}
                description={<Text type="secondary">{step.displayDescription}</Text>}
              />
            </List.Item>
          )}
        />
      </Space>
    </Card>
  );
}


