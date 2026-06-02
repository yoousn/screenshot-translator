import React from "react";
import { Alert, Descriptions, Space, Tag, Typography } from "antd";
import type { YsnOcrManagedSourceDryRunResult } from "../../ocr-models";

const { Text } = Typography;

type Labels = Record<string, string>;

interface ManagedSourceDryRunResultAlertProps {
  result: YsnOcrManagedSourceDryRunResult;
  labels: Labels;
}

export default function ManagedSourceDryRunResultAlert({ result, labels }: ManagedSourceDryRunResultAlertProps) {
  const packPlans = result.result?.packPlans || [];
  const failedPlan = packPlans.find((plan) => !plan.ok);
  const totalModels = packPlans.reduce((sum, plan) => sum + (plan.modelCount || plan.downloadPlan?.length || 0), 0);
  const firstDownload = packPlans.flatMap((plan) => plan.downloadPlan || [])[0];

  return (
    <Alert
      type={result.ok ? "success" : "warning"}
      showIcon
      message={result.ok ? labels.managedSourceDryRunPassed : labels.managedSourceDryRunBlocked}
      description={
        <Space direction="vertical" size={8} style={{ width: "100%" }}>
          <Text type="secondary">{labels.managedSourceDryRunNotReady}</Text>
          <Text type="secondary">{labels.managedSourceDryRunNextStep}</Text>
          <Descriptions size="small" column={1} bordered>
            <Descriptions.Item label={labels.managedSourceIndexPath}>{result.indexPath}</Descriptions.Item>
            <Descriptions.Item label={labels.modelPacksReadyLabel}>{packPlans.length}</Descriptions.Item>
            <Descriptions.Item label={labels.updatedModels}>{result.result?.importResult?.updatedModels?.join(", ") || "-"}</Descriptions.Item>
            <Descriptions.Item label={labels.managedSourceDryRunModelCount}>{totalModels}</Descriptions.Item>
            <Descriptions.Item label={labels.managedSourceDryRunWrites}>{result.result?.wouldWriteManifest ? labels.yes : labels.no}</Descriptions.Item>
          </Descriptions>
          {failedPlan && <Text type="danger">{labels.managedSourceDryRunBlocker}: {failedPlan.blocker}</Text>}
          {firstDownload && (
            <Space wrap>
              <Tag color="blue">{firstDownload.provider}</Tag>
              <Tag color="green">{firstDownload.license}</Tag>
              <Tag color="purple">{firstDownload.version}</Tag>
              <Tag color="default">{Math.round((firstDownload.size || 0) / 1024)} KB</Tag>
            </Space>
          )}
        </Space>
      }
    />
  );
}

