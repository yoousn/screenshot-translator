import React from "react";
import { Card, Col, Collapse, Row, Space, Typography } from "antd";
import ConfigPageHeader from "../components/config/ConfigPageHeader";
import ConfigReadinessOverview from "../components/config/ConfigReadinessOverview";
import OcrModelPackPanel from "../components/config/OcrModelPackPanel";
import RecordingDependencyPanel from "../components/config/RecordingDependencyPanel";
import TranslationLanguagePanel from "../components/config/TranslationLanguagePanel";
import useOcrConfigController from "../hooks/useOcrConfigController";
import useRecordingDependencyController from "../hooks/useRecordingDependencyController";
import useYsnOcrRuntimeController from "../hooks/useYsnOcrRuntimeController";
import { useI18n } from "../i18n";

const { Text } = Typography;

export default function OcrConfig() {
  const { config, setConfig, saveConfig } = useOcrConfigController();
  const recording = useRecordingDependencyController();
  const ysnOcrRuntime = useYsnOcrRuntimeController();
  const { text } = useI18n();
  const labels = text.config;

  const saveTargetLanguage = async (targetLang: string) => {
    setConfig({ ...config, targetLang });
    await saveConfig({ targetLang });
  };

  return (
    <Space direction="vertical" size={16} style={{ width: "100%" }}>
      <ConfigPageHeader />
      <ConfigReadinessOverview
        runtimeStatus={ysnOcrRuntime.status}
        recordingInfo={recording.recordingInfo}
        targetLang={config.targetLang || "zh"}
      />

      <Row gutter={[16, 16]}>
        <Col xs={24}>
          <OcrModelPackPanel
            runtimeStatus={ysnOcrRuntime.status}
            loadingRuntimeStatus={ysnOcrRuntime.loadingStatus}
            selfTesting={ysnOcrRuntime.selfTesting}
            runningPackAction={ysnOcrRuntime.runningPackAction}
            lastSelfTest={ysnOcrRuntime.lastSelfTest}
            lastOperation={ysnOcrRuntime.lastOperation}
            onRefreshRuntimeStatus={ysnOcrRuntime.refreshStatus}
            onRunSelfTest={ysnOcrRuntime.runSelfTest}
            onInstallPack={ysnOcrRuntime.installPack}
            onUpdatePack={ysnOcrRuntime.updatePack}
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
            defaultActiveKey={recording.recordingInfo?.ffmpegFound ? [] : ["recording"]}
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
