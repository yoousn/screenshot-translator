import { Alert, List, Space, Tag, Typography } from "antd";
import { CheckCircleOutlined, ExclamationCircleOutlined } from "@ant-design/icons";
import type { OcrActiveModelHealth } from "../../ocr-models";

const { Text } = Typography;

type ActiveModelHealthPanelProps = {
  health: OcrActiveModelHealth[];
  labels: Record<string, string>;
};

const issueText = (item: OcrActiveModelHealth, labels: Record<string, string>) => {
  if (item.ok) return labels.activeModelHealthy;
  if (!item.exists) return labels.activeModelMissingNextAction;
  if (item.issues.some((issue) => issue.code.includes("sha"))) return labels.activeModelShaNextAction;
  if (item.issues.some((issue) => issue.code.includes("source"))) return labels.activeModelSourceNextAction;
  return labels.activeModelProbeNextAction;
};

const artifactLabel = (artifactType: string | undefined, labels: Record<string, string>) => {
  if (artifactType === "dictionary") return labels.activeArtifactDictionary || "Dictionary";
  if (artifactType === "model") return labels.activeArtifactModel || "Model";
  return artifactType || labels.activeArtifactUnknown || "Artifact";
};

export default function ActiveModelHealthPanel({ health, labels }: ActiveModelHealthPanelProps) {
  if (!health.length) return null;

  const broken = health.filter((item) => !item.ok);
  const localDev = health.filter((item) => item.sourceProvider === "local-dev");

  return (
    <Space direction="vertical" size={10} style={{ width: "100%" }}>
      {broken.length > 0 && (
        <Alert
          type="error"
          showIcon
          message={labels.activeModelHealthIssueTitle}
          description={labels.activeModelHealthIssueDesc.replace("{count}", String(broken.length))}
        />
      )}
      {localDev.length > 0 && (
        <Alert
          type="info"
          showIcon
          message={labels.localDevModelsTitle}
          description={`${labels.localDevModelsDesc}: ${localDev.map((item) => `${item.modelId}:${artifactLabel(item.artifactType, labels)}`).join(", ")}`}
        />
      )}
      <List
        size="small"
        dataSource={health}
        renderItem={(item) => (
          <List.Item>
            <List.Item.Meta
              avatar={item.ok ? <CheckCircleOutlined style={{ color: "#16a34a" }} /> : <ExclamationCircleOutlined style={{ color: "#dc2626" }} />}
              title={
                <Space wrap>
                  <Text strong>{item.modelId}</Text>
                  <Tag color={item.artifactType === "dictionary" ? "blue" : "purple"}>{artifactLabel(item.artifactType, labels)}</Tag>
                  {item.packId && <Tag>{item.packId}</Tag>}
                  <Tag color={item.ok ? "green" : "red"}>{item.ok ? labels.readinessReady : labels.repairNeeded}</Tag>
                  <Tag color={item.productionSource ? "green" : "orange"}>{item.sourceProvider}</Tag>
                </Space>
              }
              description={
                <Space direction="vertical" size={2}>
                  <Text type="secondary">{item.relativePath}</Text>
                  {item.issues.length > 0 && <Text type="secondary">{item.issues.map((issue) => `${issue.code}: ${issue.message}`).join(" · ")}</Text>}
                  <Text type={item.ok ? "secondary" : "danger"}>{issueText(item, labels)}</Text>
                </Space>
              }
            />
          </List.Item>
        )}
      />
    </Space>
  );
}
