import React from "react";
import { Alert, Space, Tag, Typography } from "antd";

const { Text } = Typography;

type Labels = Record<string, string>;

interface ModelSourceStageGuideProps {
  labels: Labels;
}

export default function ModelSourceStageGuide({ labels }: ModelSourceStageGuideProps) {
  return (
    <Alert
      type="info"
      showIcon
      message={labels.modelSourceStageGuideTitle}
      description={
        <Space direction="vertical" size={6}>
          <Text type="secondary">{labels.modelSourceStageGuideDesc}</Text>
          <Space wrap>
            <Tag color="blue">1. {labels.modelSourceStageDryRun}</Tag>
            <Tag color="cyan">2. {labels.modelSourceStageImport}</Tag>
            <Tag color="purple">3. {labels.modelSourceStageInstall}</Tag>
            <Tag color="green">4. {labels.modelSourceStageSelfTest}</Tag>
          </Space>
        </Space>
      }
    />
  );
}
