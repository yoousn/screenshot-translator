import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";
import { 
  List, 
  Button, 
  Tag, 
  Space, 
  Typography, 
  Card,
  message,
  InputNumber,
  Descriptions,
  Input
} from "antd";
import { 
  HistoryOutlined, 
  DeleteOutlined, 
  PictureOutlined, 
  CheckCircleOutlined,
  ClockCircleOutlined,
  GlobalOutlined,
  FolderOpenOutlined
} from "@ant-design/icons";

const { Text, Title, Paragraph } = Typography;

interface HistoryRecord {
  id: string;
  time: string;
  filename: string;
  blocks: number;
  channel: string;
  duration: string;
  status: "success" | "warning";
}



export default function History() {
  const [history, setHistory] = useState<HistoryRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [historyInfo, setHistoryInfo] = useState<any>(null);
  const [historyMaxRecords, setHistoryMaxRecords] = useState<number>(100);
  const [historyMaxBytesMb, setHistoryMaxBytesMb] = useState<number>(2);
  const [historyDir, setHistoryDir] = useState("");

  useEffect(() => {
    loadHistory();
  }, []);

  const loadHistory = async () => {
    setLoading(true);
    try {
      const [historyStr, info] = await Promise.all([
        invoke<string>("get_history"),
        invoke<any>("get_history_info"),
      ]);
      setHistory(JSON.parse(historyStr));
      setHistoryInfo(info);
      setHistoryMaxRecords(Number(info?.maxRecords || 100));
      setHistoryMaxBytesMb(Number(((info?.maxBytes || 2 * 1024 * 1024) / 1024 / 1024).toFixed(1)));
      setHistoryDir(info?.dir || "");
    } catch (err) {
      console.error("Failed to load history:", err);
      message.error("\u52a0\u8f7d\u5386\u53f2\u8bb0\u5f55\u5931\u8d25");
    } finally {
      setLoading(false);
    }
  };

  const formatBytes = (bytes?: number) => {
    if (!bytes) return "0 KB";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / 1024 / 1024).toFixed(2) + " MB";
  };

  const saveHistoryLimits = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const config = configStr ? JSON.parse(configStr) : {};
      config.historyMaxRecords = historyMaxRecords;
      config.historyMaxBytes = Math.max(1, historyMaxBytesMb) * 1024 * 1024;
      config.historyDir = historyDir.trim();
      await invoke("save_config", { configStr: JSON.stringify(config) });
      message.success("历史记录配置已保存");
      loadHistory();
    } catch (error: any) {
      message.error("保存历史记录限制失败：" + (error?.message || error));
    }
  };

  const handleClearHistory = async () => {
    try {
      await invoke("clear_history");
      setHistory([]);
      loadHistory();
      message.success("已清空历史记录");
    } catch (err) {
      message.error("清空历史记录失败");
    }
  };

  const chooseHistoryDir = async () => {
    try {
      const dir = await invoke<string | null>("choose_history_dir", { currentDir: historyDir });
      if (dir) setHistoryDir(dir);
    } catch (error: any) {
      message.error("\u9009\u62e9\u5386\u53f2\u76ee\u5f55\u5931\u8d25\uff1a" + (error?.message || error));
    }
  };

  const openHistoryDir = async () => {
    try {
      if (historyDir) await openPath(historyDir);
    } catch (error: any) {
      message.error("\u6253\u5f00\u5386\u53f2\u76ee\u5f55\u5931\u8d25\uff1a" + (error?.message || error));
    }
  };

  return (
    <Card bordered={false} style={{ borderRadius: 12, boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderBottom: "1px solid #e8e8e8", paddingBottom: 16, marginBottom: 24 }}>
        <div>
          <Title level={4} style={{ margin: 0 }}>
            历史翻译记录
          </Title>
          <Paragraph type="secondary" style={{ fontSize: 12, margin: "4px 0 0 0" }}>
            查看本地及网络端发起的历史翻译审计日志。
          </Paragraph>
        </div>
        <Button
          type="default"
          icon={<DeleteOutlined />}
          onClick={handleClearHistory}
          style={{ height: 32 }}
        >
          清理历史记录
        </Button>
      </div>


      {historyInfo && (
        <Card size="small" style={{ marginBottom: 16, background: "#fafafa" }}>
          <Descriptions size="small" column={1} bordered>
            <Descriptions.Item label="历史文件路径">{historyInfo.path}</Descriptions.Item>
            <Descriptions.Item label="当前数量">{historyInfo.count} 条</Descriptions.Item>
            <Descriptions.Item label="当前占用">{formatBytes(historyInfo.bytes)}</Descriptions.Item>
          </Descriptions>
          <Space style={{ marginTop: 12 }} wrap>
            <span>历史目录</span>
            <Input value={historyDir} onChange={(event) => setHistoryDir(event.target.value)} style={{ width: 360 }} placeholder="默认目录" />
            <Button icon={<FolderOpenOutlined />} onClick={chooseHistoryDir}>选择目录</Button>
            <Button onClick={openHistoryDir}>打开目录</Button>
            <Button onClick={() => setHistoryDir("")}>恢复默认目录</Button>
          </Space>
          <Space style={{ marginTop: 12 }} wrap>
            <span>最大数量</span>
            <InputNumber min={10} max={5000} value={historyMaxRecords} onChange={(value) => setHistoryMaxRecords(Number(value || 100))} />
            <span>最大大小(MB)</span>
            <InputNumber min={1} max={100} value={historyMaxBytesMb} onChange={(value) => setHistoryMaxBytesMb(Number(value || 2))} />
            <Button type="primary" onClick={saveHistoryLimits}>保存配置</Button>
          </Space>
        </Card>
      )}

      <div style={{ marginBottom: 12 }}>
        <Text strong style={{ fontSize: 12, display: "flex", alignItems: "center", gap: 6, color: "#1f1f1f" }}>
          <HistoryOutlined style={{ color: "#1677ff" }} />
          历史事件流水线 (Audit Logs)
        </Text>
      </div>

      <List
        itemLayout="horizontal"
        dataSource={history}
        loading={loading}
        locale={{ emptyText: "暂无历史翻译记录" }}
        renderItem={(item) => (
          <List.Item
            style={{
              padding: "16px 20px",
              border: "1px solid #f0f0f0",
              borderRadius: 12,
              marginBottom: 10,
              background: "#ffffff",
            }}
            actions={[
              <Space size="large" key="meta">
                <Text style={{ fontSize: 11, color: "#595959" }}>
                  识别块: <b>{item.blocks} 个</b>
                </Text>
                <Text style={{ fontSize: 11, color: "#595959" }}>
                  耗时: <b>{item.duration}</b>
                </Text>
                <Tag color="success" icon={<CheckCircleOutlined />} style={{ margin: 0 }}>
                  已翻译
                </Tag>
              </Space>
            ]}
          >
            <List.Item.Meta
              avatar={<PictureOutlined style={{ fontSize: 18, color: "#1677ff", background: "#e6f7ff", padding: 8, borderRadius: 8 }} />}
              title={
                <Text strong style={{ fontSize: 13, color: "#1f1f1f" }}>
                  {item.filename}
                </Text>
              }
              description={
                <Space size="middle" style={{ fontSize: 10, color: "#8c8c8c", marginTop: 2 }}>
                  <span>
                    <ClockCircleOutlined style={{ marginRight: 4 }} />
                    {item.time}
                  </span>
                  <span>
                    <GlobalOutlined style={{ marginRight: 4 }} />
                    通道: {item.channel}
                  </span>
                </Space>
              }
            />
          </List.Item>
        )}
      />
    </Card>
  );
}
