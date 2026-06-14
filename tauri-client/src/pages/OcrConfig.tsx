import React from "react";
import { Card, Col, Row, Space, Tag, Typography } from "antd";
import RecordingDependencyPanel from "../components/config/RecordingDependencyPanel";
import TranslationLanguagePanel from "../components/config/TranslationLanguagePanel";
import useOcrConfigController from "../hooks/useOcrConfigController";
import useRecordingDependencyController from "../hooks/useRecordingDependencyController";

const { Text } = Typography;

export default function OcrConfig() {
  const { config, setConfig, saveConfig } = useOcrConfigController();
  const recording = useRecordingDependencyController();

  const saveTargetLanguage = async (targetLang: string) => {
    setConfig({ ...config, targetLang });
    await saveConfig({ targetLang });
  };

  return (
    <Space orientation="vertical" size={16} style={{ width: "100%" }}>
      <Card variant="borderless" style={{ borderRadius: 20, background: "linear-gradient(135deg, #eef6ff 0%, #f8fbff 56%, #fff7ed 100%)" }}>
        <Space orientation="vertical" size={10} style={{ width: "100%" }}>
          <Space wrap>
            <Tag color="orange">FFmpeg</Tag>
            <Tag color="blue">目标语言</Tag>
          </Space>
          <div>
            <Text strong style={{ display: "block", fontSize: 22, color: "#0f172a" }}>视频录制 / 翻译目标</Text>
            <Text type="secondary" style={{ display: "block", marginTop: 6 }}>管理 FFmpeg 录制依赖、录屏保存目录和默认翻译目标语言。</Text>
            <Text type="secondary" style={{ display: "block", marginTop: 8, fontSize: 12 }}>
              识字模型下载和安装已移到左侧“模型管理”，这里专注录制和语言设置。
            </Text>
          </div>
        </Space>
      </Card>

      <Row gutter={[16, 16]} align="stretch">
        <Col xs={24} xl={12}>
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
        </Col>
        <Col xs={24} xl={12}>
          <TranslationLanguagePanel targetLang={config.targetLang || "zh"} onTargetLangChange={saveTargetLanguage} />
        </Col>
      </Row>
    </Space>
  );
}
