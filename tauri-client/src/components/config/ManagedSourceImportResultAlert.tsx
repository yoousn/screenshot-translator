import { Alert, Space, Tag, Typography } from "antd";
import type { YsnOcrManagedSourceImportResult } from "../../ocr-models";

const { Text } = Typography;

type ManagedSourceImportResultAlertProps = {
  result: YsnOcrManagedSourceImportResult;
  labels: Record<string, string>;
};

export default function ManagedSourceImportResultAlert({ result, labels }: ManagedSourceImportResultAlertProps) {
  const readiness = result.sourceReadiness;
  const ready = Boolean(readiness?.ready);
  const pendingModelIds = readiness?.pendingModelIds || [];
  const firstIssue = readiness?.issues?.[0];

  return (
    <Alert
      type={ready ? "success" : "warning"}
      showIcon
      message={labels.managedSourceImportResult.replace("{count}", String(result.updatedCount || 0))}
      description={
        <Space direction="vertical" size={4}>
          <Text type="secondary">{labels.managedSourceIndexPath}: {result.indexPath}</Text>
          <Text type="secondary">{labels.trustedSourcesConfigured}: {readiness?.configuredModels || 0} / {readiness?.requiredModels || 0}</Text>
          {result.updatedModels?.length > 0 && <Text type="secondary">{labels.updatedModels}: {result.updatedModels.join(", ")}</Text>}
          <Space wrap>
            <Tag color={ready ? "green" : "orange"}>{ready ? labels.managedSourceNextInstallPacks : labels.managedSourceNextFixSources}</Tag>
            {pendingModelIds.length > 0 && <Tag color="orange">{labels.trustedSourcesPendingModels}: {pendingModelIds.slice(0, 6).join(", ")}</Tag>}
          </Space>
          {firstIssue && <Text type="secondary">{labels.trustedSourcesFirstIssue}: {firstIssue.message}</Text>}
        </Space>
      }
    />
  );
}
