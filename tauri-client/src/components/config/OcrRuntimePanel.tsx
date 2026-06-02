import React from "react";
import { Alert, Button, Descriptions, Input, InputNumber, Space, Tag, Typography } from "antd";
import { FolderOpenOutlined, SaveOutlined, SafetyCertificateOutlined } from "@ant-design/icons";
import ConfigSectionCard from "./ConfigSectionCard";
import type { OcrRuntimePanelProps } from "./types";
import { useI18n } from "../../i18n";

const { Text } = Typography;

export default function OcrRuntimePanel({
  config,
  status,
  statusTag,
  saving,
  checkingStatus,
  onSetConfig,
  onSaveConfig,
  onChooseRuntimeDir,
  onCheckStatus,
  onOpenRuntimeDir,
}: OcrRuntimePanelProps) {
  const { text } = useI18n();
  const labels = text.config;
  const manifest = status?.runtimeManifest;
  const engine = (manifest?.engine || (status?.path?.toLowerCase().includes("paddle") ? "paddleocr-json" : "ysn-ocr-runtime")).toString();
  const isOwnedRuntime = engine.toLowerCase().includes("ysn") || engine.toLowerCase().includes("rapid") || engine.toLowerCase().includes("onnx");

  return (
    <ConfigSectionCard eyebrow={labels.ocrRuntimeEyebrow} title={labels.ocrRuntimeTitle} description={labels.ocrRuntimeDesc}>
      <Alert
        type={isOwnedRuntime ? "info" : "warning"}
        showIcon
        message={isOwnedRuntime ? labels.commercialOcrMainlineMessage : labels.compatibilityRuntimeDetected}
        description={labels.ocrRuntimeInfoDesc}
      />
      <div>
        <Text strong>{labels.runtimePath}</Text>
        <Input
          style={{ marginTop: 8 }}
          placeholder={labels.runtimePathPlaceholder}
          value={config.localOcrExecutablePath || ""}
          onChange={(event) => onSetConfig({ ...config, localOcrExecutablePath: event.target.value })}
        />
      </div>
      <div>
        <Text strong>{labels.localOcrTimeout}</Text>
        <InputNumber
          style={{ marginTop: 8, width: "100%" }}
          min={3000}
          max={120000}
          step={1000}
          value={config.localOcrTimeoutMs || 15000}
          onChange={(value) => onSetConfig({ ...config, localOcrTimeoutMs: Number(value || 15000) })}
        />
      </div>
      <Space wrap>
        <Button type="primary" icon={<SaveOutlined />} loading={saving} onClick={onSaveConfig}>{labels.saveOcrConfig}</Button>
        <Button icon={<FolderOpenOutlined />} onClick={onChooseRuntimeDir}>{labels.chooseRuntimeFolder}</Button>
        <Button icon={<SafetyCertificateOutlined />} loading={checkingStatus} onClick={onCheckStatus}>{labels.checkRuntime}</Button>
        <Button icon={<FolderOpenOutlined />} disabled={!config.localOcrExecutablePath && !status?.path} onClick={onOpenRuntimeDir}>{labels.openRuntime}</Button>
      </Space>
      <Descriptions size="small" column={1} bordered>
        <Descriptions.Item label={labels.status}>{statusTag}</Descriptions.Item>
        <Descriptions.Item label={labels.sourceLanguage}><Tag color="blue">{labels.sourceLanguageAuto}</Tag></Descriptions.Item>
        <Descriptions.Item label={labels.recommendedMode}>{isOwnedRuntime ? <Tag color="blue">{labels.ownedRuntimeMode}</Tag> : <Tag color="orange">{labels.compatibilityMode}</Tag>}</Descriptions.Item>
        <Descriptions.Item label={labels.resolvedPath}>{status?.path || labels.autoDetect}</Descriptions.Item>
        <Descriptions.Item label={labels.runtimeName}>{manifest?.name || (isOwnedRuntime ? "YSN OCR Runtime" : "PaddleOCR-json")}</Descriptions.Item>
        <Descriptions.Item label={labels.engine}>{engine}</Descriptions.Item>
        <Descriptions.Item label={labels.protocol}>{manifest?.protocol || (isOwnedRuntime ? "managed-model-pack" : "paddleocr-json-stdin")}</Descriptions.Item>
        <Descriptions.Item label={labels.version}>{manifest?.version || labels.planned}</Descriptions.Item>
      </Descriptions>
    </ConfigSectionCard>
  );
}
