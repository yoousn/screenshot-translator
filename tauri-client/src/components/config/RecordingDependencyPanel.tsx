import { Alert, Button, Card, Descriptions, Input, Progress, Space, Tag, Typography } from "antd";
import { CloudDownloadOutlined, FolderOpenOutlined, ReloadOutlined, VideoCameraOutlined } from "@ant-design/icons";
import type { RecordingDependencyPanelProps } from "./types";

const { Text } = Typography;

export default function RecordingDependencyPanel({
  ffmpegPath,
  defaultVideoDir,
  ffmpegProgress,
  recordingInfo,
  checkingRecordingInfo,
  downloadingFfmpeg,
  onSetFfmpegPath,
  onChooseFfmpegPath,
  onCheckRecordingInfo,
  onDownloadFfmpeg,
  onOpenFfmpegDir,
  onOpenVideoDir,
}: RecordingDependencyPanelProps) {
  const ready = Boolean(recordingInfo?.ffmpegFound);
  const audioCount = recordingInfo?.audioDevices?.length || 0;

  return (
    <Card
      variant="borderless"
      title={<span><VideoCameraOutlined style={{ marginRight: 8 }} />视频录制</span>}
      extra={<Button size="small" icon={<ReloadOutlined />} loading={checkingRecordingInfo} onClick={onCheckRecordingInfo}>重新检测</Button>}
      style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.06)" }}
    >
      <Space orientation="vertical" size={14} style={{ width: "100%" }}>
        <Alert
          type={ready ? "success" : "warning"}
          showIcon
          title={ready ? "FFmpeg 可用" : "FFmpeg 缺失"}
          description={ready ? "录屏功能已可使用，录制结果会自动保存到 Videos\\YSN。" : "录屏前需要下载 FFmpeg，或选择本机已有的 ffmpeg.exe。"}
          action={<Button type={ready ? "default" : "primary"} icon={<CloudDownloadOutlined />} loading={downloadingFfmpeg} onClick={onDownloadFfmpeg}>下载 FFmpeg</Button>}
        />

        <Input
          value={ffmpegPath}
          placeholder="自动检测，或选择 ffmpeg.exe"
          onChange={(event) => onSetFfmpegPath(event.target.value)}
        />

        <Space wrap>
          <Button icon={<FolderOpenOutlined />} onClick={onChooseFfmpegPath}>选择 ffmpeg.exe</Button>
          <Button disabled={!ffmpegPath} onClick={onOpenFfmpegDir}>打开 FFmpeg 目录</Button>
          <Button disabled={!defaultVideoDir} onClick={onOpenVideoDir}>打开录屏目录</Button>
          <Tag color={ready ? "green" : "orange"}>{ready ? "已就绪" : "待处理"}</Tag>
        </Space>

        <Descriptions size="small" column={1} bordered>
          <Descriptions.Item label="检测路径">{recordingInfo?.ffmpegPath || ffmpegPath || "未检测到"}</Descriptions.Item>
          <Descriptions.Item label="录屏目录">{defaultVideoDir || "Videos\\YSN"}</Descriptions.Item>
          <Descriptions.Item label="音频设备">{audioCount > 0 ? `${audioCount} 个设备` : "未检测或无设备"}</Descriptions.Item>
        </Descriptions>

        {ffmpegProgress && (
          <Progress
            percent={ffmpegProgress.percent}
            status={ffmpegProgress.percent >= 100 ? "success" : "active"}
            format={() => `${ffmpegProgress.phase} ${ffmpegProgress.percent}%`}
          />
        )}

        <Text type="secondary" style={{ fontSize: 12 }}>
          FFmpeg 只负责视频录制；截图、识字和翻译不依赖它。
        </Text>
      </Space>
    </Card>
  );
}
