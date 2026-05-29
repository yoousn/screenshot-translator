import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { 
  List, 
  Button, 
  Tag, 
  Space, 
  Typography, 
  Card,
  message 
} from "antd";
import { 
  HistoryOutlined, 
  DeleteOutlined, 
  PictureOutlined, 
  CheckCircleOutlined,
  ClockCircleOutlined,
  GlobalOutlined
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

  useEffect(() => {
    loadHistory();
  }, []);

  const loadHistory = async () => {
    setLoading(true);
    try {
      const historyStr = await invoke<string>("get_history");
      setHistory(JSON.parse(historyStr));
    } catch (err) {
      console.error("Failed to load history:", err);
      message.error("加载历史记录失败");
    } finally {
      setLoading(false);
    }
  };

  const handleClearHistory = async () => {
    try {
      await invoke("clear_history");
      setHistory([]);
      message.success("已清空历史记录");
    } catch (err) {
      message.error("清空历史记录失败");
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
