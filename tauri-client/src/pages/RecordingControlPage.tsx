import React from "react";
import { App as AntdApp, ConfigProvider } from "antd";
import RecordingControlHud from "../components/recording/RecordingControlHud";
import { useRecordingControl } from "../hooks/useRecordingControl";

const formatRecordingTime = (ms: number) => {
  const totalSeconds = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(totalSeconds / 3600).toString().padStart(2, "0");
  const minutes = Math.floor((totalSeconds % 3600) / 60).toString().padStart(2, "0");
  const seconds = (totalSeconds % 60).toString().padStart(2, "0");
  return `${hours}:${minutes}:${seconds}`;
};

function RecordingControlContent() {
  const {
    status,
    elapsedMs,
    countdown,
    busy,
    sessionReady,
    savedPath,
    audioLabel,
    toggleRecord,
    pauseRecording,
    resumeRecording,
    cancelRecording,
    openVideoFolder,
    copySavedVideo,
  } = useRecordingControl();

  return (
    <RecordingControlHud
      status={status}
      elapsedText={formatRecordingTime(elapsedMs)}
      countdown={countdown}
      busy={busy}
      sessionReady={sessionReady}
      hasSavedVideo={Boolean(savedPath)}
      audioLabel={audioLabel}
      onToggleRecord={toggleRecord}
      onPause={pauseRecording}
      onResume={resumeRecording}
      onOpenFolder={openVideoFolder}
      onCopy={copySavedVideo}
      onCancel={cancelRecording}
    />
  );
}

export default function RecordingControlPage() {
  return (
    <ConfigProvider theme={{ token: { borderRadius: 12, colorPrimary: "#2563eb" } }}>
      <AntdApp>
        <RecordingControlContent />
      </AntdApp>
    </ConfigProvider>
  );
}
