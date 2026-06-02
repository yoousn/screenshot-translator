import React from "react";
import { Alert, Button, Descriptions, Space, Tag, Typography } from "antd";
import { ApiOutlined, ExperimentOutlined, FileTextOutlined, ImportOutlined, ReloadOutlined } from "@ant-design/icons";
import ActiveModelHealthPanel from "./ActiveModelHealthPanel";
import ConfigSectionCard from "./ConfigSectionCard";
import ManagedSourceDryRunResultAlert from "./ManagedSourceDryRunResultAlert";
import ManagedSourceImportResultAlert from "./ManagedSourceImportResultAlert";
import ModelPackOperationStatus from "./ModelPackOperationStatus";
import ModelPackStatusList from "./ModelPackStatusList";
import ModelSourceStageGuide from "./ModelSourceStageGuide";
import OcrRuntimeReadinessSteps from "./OcrRuntimeReadinessSteps";
import { getDefaultOcrModelManifest } from "../../ocr-models";
import type { OcrModelPackOperation, YsnOcrManagedSourceDryRunResult, YsnOcrManagedSourceImportResult, YsnOcrManagedSourceTemplateResult, YsnOcrRuntimeStatus, YsnOcrSelfTestResult } from "../../ocr-models";
import { useI18n } from "../../i18n";
import type { StatusResult } from "../../utils/ocrConfigHelpers";

const { Text } = Typography;

interface OcrModelPackPanelProps {
  status: StatusResult | null;
  runtimeStatus: YsnOcrRuntimeStatus | null;
  loadingRuntimeStatus: boolean;
  selfTesting: boolean;
  importingManagedSources?: boolean;
  dryRunningManagedSources?: boolean;
  creatingManagedSourceTemplate?: boolean;
  runningPackAction?: string | null;
  lastSelfTest?: YsnOcrSelfTestResult | null;
  lastOperation?: OcrModelPackOperation | null;
  lastManagedSourceImport?: YsnOcrManagedSourceImportResult | null;
  lastManagedSourceDryRun?: YsnOcrManagedSourceDryRunResult | null;
  lastManagedSourceTemplate?: YsnOcrManagedSourceTemplateResult | null;
  onRefreshRuntimeStatus: () => void;
  onRunSelfTest: () => void;
  onImportManagedSourceIndex: () => void;
  onDryRunManagedSourceIndex: () => void;
  onCreateManagedSourceTemplate: () => void;
  onInstallPack: (packId: string) => void;
  onUpdatePack: (packId: string) => void;
}

export default function OcrModelPackPanel({
  status,
  runtimeStatus,
  loadingRuntimeStatus,
  selfTesting,
  importingManagedSources,
  dryRunningManagedSources,
  creatingManagedSourceTemplate,
  runningPackAction,
  lastSelfTest,
  lastOperation,
  lastManagedSourceImport,
  lastManagedSourceDryRun,
  lastManagedSourceTemplate,
  onRefreshRuntimeStatus,
  onRunSelfTest,
  onImportManagedSourceIndex,
  onDryRunManagedSourceIndex,
  onCreateManagedSourceTemplate,
  onInstallPack,
  onUpdatePack,
}: OcrModelPackPanelProps) {
  const { text } = useI18n();
  const labels = text.config;
  const manifest = status?.runtimeManifest;
  const modelManifest = runtimeStatus?.manifest || getDefaultOcrModelManifest();
  const manifestIssues = runtimeStatus?.manifestIssues || [];
  const blockingIssues = manifestIssues.filter((issue) => issue.severity === "error");
  const selfTestIssues = lastSelfTest?.manifestIssues || [];
  const missingActiveModels = lastSelfTest?.missingActiveModels || [];
  const sourceReadiness = runtimeStatus?.sourceReadiness;
  const sourceIssues = sourceReadiness?.issues || [];
  const sourcePolicy = sourceReadiness?.policy || modelManifest.sourcePolicy;
  const activeModelHealth = runtimeStatus?.activeModelHealth || [];
  const languages = manifest?.languages?.length ? manifest.languages.join(", ") : modelManifest.packs[0]?.languages.join(", ");

  return (
    <ConfigSectionCard
      eyebrow={labels.modelPacksEyebrow}
      title={<span><ApiOutlined style={{ marginRight: 8 }} />{labels.modelPacksTitle}</span>}
      description={labels.modelPacksDesc}
      extra={<Space wrap><Button size="small" icon={<FileTextOutlined />} loading={creatingManagedSourceTemplate} onClick={onCreateManagedSourceTemplate}>{labels.createManagedSourceTemplate}</Button><Button size="small" icon={<ImportOutlined />} loading={dryRunningManagedSources} onClick={onDryRunManagedSourceIndex}>{labels.dryRunManagedSourceIndex}</Button><Button size="small" icon={<ImportOutlined />} loading={importingManagedSources} onClick={onImportManagedSourceIndex}>{labels.importManagedSourceIndex}</Button><Button size="small" icon={<ReloadOutlined />} loading={loadingRuntimeStatus} onClick={onRefreshRuntimeStatus}>{labels.refresh}</Button><Button size="small" icon={<ExperimentOutlined />} loading={selfTesting} onClick={onRunSelfTest}>{labels.selfTest}</Button></Space>}
    >
      <Alert type={runtimeStatus?.ready ? "success" : "info"} showIcon message={runtimeStatus?.ready ? labels.runtimeReady : labels.designedForOwnership} description={runtimeStatus?.implementationStatus || labels.noRuntimeDetails} />
      <Space wrap>
        <Tag color={runtimeStatus?.modelPacksReady ? "green" : "orange"}>{labels.modelPacksReadyLabel}: {runtimeStatus?.modelPacksReady ? labels.modelPacksReady : labels.modelPacksNotReady}</Tag>
        <Tag color={runtimeStatus?.runtimeInferenceReady ? "green" : "blue"}>{labels.onnxInferenceLabel}: {runtimeStatus?.runtimeInferenceReady ? labels.onnxInferenceEnabled : labels.onnxInferenceNotEnabled}</Tag>
        <Tag color={sourceReadiness?.ready ? "green" : "orange"}>{labels.trustedSourcesLabel}: {sourceReadiness?.ready ? labels.trustedSourcesReady : labels.trustedSourcesPending}</Tag>
        <Tag color={runtimeStatus?.ready ? "green" : "default"}>{labels.runtimeReadyLabel}: {runtimeStatus?.ready ? labels.yes : labels.no}</Tag>
      </Space>
      <ModelSourceStageGuide labels={labels} />
      <OcrRuntimeReadinessSteps steps={runtimeStatus?.readinessSteps} labels={labels} />
      {sourceReadiness && !sourceReadiness.ready && (
        <Alert
          type="warning"
          showIcon
          message={labels.trustedSourcesPendingTitle}
          description={
            <Space direction="vertical" size={2}>
              <Text type="secondary">{labels.trustedSourcesPendingDesc}</Text>
              <Text type="secondary">{labels.trustedSourcesConfigured}: {sourceReadiness.configuredModels} / {sourceReadiness.requiredModels}</Text>
              {sourceReadiness.pendingModelIds.length > 0 && <Text type="secondary">{labels.trustedSourcesPendingModels}: {sourceReadiness.pendingModelIds.slice(0, 6).join(", ")}</Text>}
              {sourceIssues.length > 0 && <Text type="secondary">{labels.trustedSourcesFirstIssue}: {sourceIssues[0].message}</Text>}
            </Space>
          }
        />
      )}
      {lastManagedSourceDryRun && (
        <ManagedSourceDryRunResultAlert result={lastManagedSourceDryRun} labels={labels} />
      )}
      {lastManagedSourceImport && (
        <ManagedSourceImportResultAlert result={lastManagedSourceImport} labels={labels} />
      )}
      {lastManagedSourceTemplate && (
        <Alert
          type="info"
          showIcon
          message={labels.managedSourceTemplateResult.replace("{count}", String(lastManagedSourceTemplate.modelCount || 0))}
          description={
            <Space direction="vertical" size={2}>
              <Text type="secondary">{labels.managedSourceTemplatePath}: {lastManagedSourceTemplate.templatePath}</Text>
              <Text type="secondary">{labels.managedSourceTemplateDir}: {lastManagedSourceTemplate.templateDir}</Text>
            </Space>
          }
        />
      )}
      <ActiveModelHealthPanel health={activeModelHealth} labels={labels} />
      {manifestIssues.length > 0 && (
        <Alert
          type={blockingIssues.length > 0 ? "error" : "warning"}
          showIcon
          message={`${labels.modelManifestIssues}: ${blockingIssues.length} ${labels.errors} / ${manifestIssues.length - blockingIssues.length} ${labels.warnings}`}
          description={manifestIssues.slice(0, 3).map((issue) => issue.message).join(" ? ")}
        />
      )}
      <Descriptions size="small" column={1} bordered>
        <Descriptions.Item label={labels.manifestSource}>{runtimeStatus ? <Tag color="green">{labels.backendStatusConnected}</Tag> : manifest ? <Tag color="green">{labels.runtimeManifestDetected}</Tag> : <Tag color="blue">{labels.plannedManagedManifest}</Tag>}</Descriptions.Item>
        <Descriptions.Item label={labels.runtimeEngine}>{manifest?.engine || runtimeStatus?.runtime || "YSN OCR Runtime / ONNX Runtime"}</Descriptions.Item>
        <Descriptions.Item label={labels.runtimeVersion}>{runtimeStatus?.runtimeVersion || "0.1.0-planned"}</Descriptions.Item>
        <Descriptions.Item label={labels.modelDirectory}>{runtimeStatus?.modelDir || labels.appDataModelsOcr}</Descriptions.Item>
        <Descriptions.Item label={labels.modelPackVerification}>{labels.modelPackVerificationValue}</Descriptions.Item>
        <Descriptions.Item label={labels.trustedSourcePolicy}>{sourcePolicy?.policyVersion || labels.notConfigured}</Descriptions.Item>
        <Descriptions.Item label={labels.trustedSourceProvider}>{sourcePolicy?.productionDownloadProvider || labels.notConfigured}</Descriptions.Item>
        <Descriptions.Item label={labels.trustedSourceNextAction}>{sourceReadiness?.nextAction || labels.configureManagedModelSources}</Descriptions.Item>
        <Descriptions.Item label={labels.languageCoverage}>{languages}</Descriptions.Item>
        <Descriptions.Item label={labels.lastSelfTest}>{lastSelfTest?.testedAt || modelManifest.lastSelfTestAt || labels.notTested}</Descriptions.Item>
        <Descriptions.Item label={labels.fallbackPolicy}>{labels.fallbackPolicyValue}</Descriptions.Item>
      </Descriptions>
      {lastSelfTest && (
        <Alert
          type={lastSelfTest.ok ? "success" : lastSelfTest.modelPacksReady ? "info" : "warning"}
          showIcon
          message={lastSelfTest.message}
          description={
            <Space direction="vertical" size={2}>
              <Text type="secondary">{labels.lastSelfTestLabel}: {lastSelfTest.testedAt}</Text>
              {selfTestIssues.length > 0 && <Text type="secondary">{labels.manifestIssueCount}: {selfTestIssues.length}</Text>}
              {missingActiveModels.length > 0 && <Text type="danger">{labels.missingActiveModels}: {missingActiveModels.join(", ")}</Text>}
            </Space>
          }
        />
      )}
      <ModelPackOperationStatus operation={lastOperation || null} />
      <ModelPackStatusList
        manifest={modelManifest}
        actionLoadingPackId={runningPackAction}
        selfTesting={selfTesting}
        sourceReadiness={sourceReadiness}
        onInstallPack={onInstallPack}
        onUpdatePack={onUpdatePack}
        onSelfTest={onRunSelfTest}
      />
      <Space wrap>
        <Tag color="blue">{labels.detector}</Tag>
        <Tag color="purple">{labels.recognizerPool}</Tag>
        <Tag color="cyan">{labels.scriptRouter}</Tag>
        <Tag color="gold">{labels.confidenceScorer}</Tag>
        <Tag color="green">{labels.selfTest}</Tag>
      </Space>
    </ConfigSectionCard>
  );
}


