import React from "react";
import { Card, Col, Collapse, Row, Space, Typography } from "antd";
import ConfigPageHeader from "../components/config/ConfigPageHeader";
import ConfigReadinessOverview from "../components/config/ConfigReadinessOverview";
import RapidOcrPanel from "../components/config/RapidOcrPanel";
import RecordingDependencyPanel from "../components/config/RecordingDependencyPanel";
import TranslationLanguagePanel from "../components/config/TranslationLanguagePanel";
import useOcrConfigController from "../hooks/useOcrConfigController";
import useRapidOcrController from "../hooks/useRapidOcrController";
import useRecordingDependencyController from "../hooks/useRecordingDependencyController";
import { useI18n } from "../i18n";
import type { RapidOcrModelVersion } from "../ocr-models";

const { Text } = Typography;

export default function OcrConfig() {
  const { config, setConfig, saveConfig } = useOcrConfigController();
  const recording = useRecordingDependencyController();
  const rapidOcr = useRapidOcrController();
  const { text } = useI18n();
  const labels = text.config;

  const saveTargetLanguage = async (targetLang: string) => {
    setConfig({ ...config, targetLang });
    await saveConfig({ targetLang });
  };

  const saveRapidOcrModelVersion = async (rapidOcrModelVersion: RapidOcrModelVersion) => {
    setConfig({ ...config, rapidOcrModelVersion });
    await saveConfig({ rapidOcrModelVersion }, false);
    await rapidOcr.refreshStatus();
  };

  const saveRapidOcrWorkerEnabled = async (rapidOcrWorkerEnabled: boolean) => {
    setConfig({ ...config, rapidOcrWorkerEnabled });
    await saveConfig({ rapidOcrWorkerEnabled }, false);
    if (rapidOcrWorkerEnabled) {
      await rapidOcr.startWorker();
    } else {
      await rapidOcr.stopWorker();
    }
    await rapidOcr.refreshStatus();
  };

  return (
    <Space direction="vertical" size={16} style={{ width: "100%" }}>
      <ConfigPageHeader />
      <ConfigReadinessOverview
        runtimeStatus={rapidOcr.status}
        recordingInfo={recording.recordingInfo}
        targetLang={config.targetLang || "zh"}
      />

      <Row gutter={[16, 16]}>
        <Col xs={24}>
          <RapidOcrPanel
            status={rapidOcr.status}
            loadingStatus={rapidOcr.loadingStatus}
            selfTesting={rapidOcr.selfTesting}
            workerBusy={rapidOcr.workerBusy}
            modelVersion={(config.rapidOcrModelVersion || "v5") as RapidOcrModelVersion}
            workerEnabled={config.rapidOcrWorkerEnabled !== false}
            lastSelfTest={rapidOcr.lastSelfTest}
            onModelVersionChange={saveRapidOcrModelVersion}
            onWorkerEnabledChange={saveRapidOcrWorkerEnabled}
            onRefreshStatus={rapidOcr.refreshStatus}
            onRunSelfTest={rapidOcr.runSelfTest}
            onStartWorker={rapidOcr.startWorker}
            onStopWorker={rapidOcr.stopWorker}
            onRestartWorker={rapidOcr.restartWorker}
          />
        </Col>
      </Row>

      <TranslationLanguagePanel targetLang={config.targetLang || "zh"} onTargetLangChange={saveTargetLanguage} />

      <Card bordered={false} style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}>
        <Space direction="vertical" size={12} style={{ width: "100%" }}>
          <div>
            <Text strong style={{ display: "block", color: "#0f172a", fontSize: 16 }}>{labels.advancedDependencies}</Text>
            <Text type="secondary" style={{ fontSize: 12 }}>{labels.advancedDependenciesDesc}</Text>
          </div>
          <Collapse
            ghost
            defaultActiveKey={[]}
            items={[
              {
                key: "recording",
                label: labels.videoRecordingDependencyPanel,
                children: (
                  <RecordingDependencyPanel
                    ffmpegPath={recording.ffmpegPath}
                    defaultVideoDir={recording.defaultVideoDir}
                    ffmpegRelease={recording.ffmpegRelease}
                    ffmpegProgress={recording.ffmpegProgress}
                    recordingInfo={recording.recordingInfo}
                    checkingFfmpeg={recording.checkingFfmpeg}
                    checkingRecordingInfo={recording.checkingRecordingInfo}
                    downloadingFfmpeg={recording.downloadingFfmpeg}
                    onSetFfmpegPath={recording.setFfmpegPath}
                    onSaveFfmpegPath={() => recording.saveFfmpegPath()}
                    onChooseFfmpegPath={recording.chooseFfmpegPath}
                    onCheckFfmpegRelease={recording.checkFfmpegRelease}
                    onCheckRecordingInfo={recording.checkRecordingInfo}
                    onDownloadFfmpeg={recording.downloadFfmpegRelease}
                    onOpenFfmpegRepo={recording.openFfmpegRepo}
                    onOpenFfmpegDir={recording.openFfmpegDir}
                    onOpenVideoDir={recording.openVideoDir}
                  />
                ),
              },
            ]}
          />
        </Space>
      </Card>
    </Space>
  );
}
