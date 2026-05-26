import React from "react";
import { 
  Card, 
  Typography, 
  Row, 
  Col, 
  Space, 
  Tag, 
  Button 
} from "antd";
import { 
  InfoCircleOutlined, 
  GithubOutlined, 
  HeartOutlined, 
  ThunderboltOutlined, 
  SafetyCertificateOutlined 
} from "@ant-design/icons";

const { Title, Paragraph, Text } = Typography;

export default function About() {
  const creditsList = [
    { name: "Tauri 2.0 Core Runtime (Rust Desktop)", desc: "提供超轻量本地系统桥接与 IPC 管道驱动" },
    { name: "React 19 + TypeScript compiler", desc: "构建高安全性、强类型的前端模块系统" },
    { name: "Ant Design v5 Layout", desc: "企业级高可交互桌面工具组件规范" },
    { name: "PaddleOCR-json open pipeline", desc: "本地离线高精度 OCR 图像识别加速引擎" },
  ];

  return (
    <Space direction="vertical" size="large" style={{ width: "100%", maxWidth: 800, margin: "0 auto" }}>
      {/* Page Title Header */}
      <div style={{ borderBottom: "1px solid #e8e8e8", paddingBottom: 16, marginBottom: 8 }}>
        <Title level={4} style={{ margin: 0 }}>
          关于系统
        </Title>
        <Paragraph type="secondary" style={{ fontSize: 12, margin: "4px 0 0 0" }}>
          了解 YSN 截图翻译客户端的后台加速引擎与底层硬件重构。
        </Paragraph>
      </div>

      {/* Main Info Card */}
      <Card bordered={false} style={{ borderRadius: 12, boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}>
        <Space direction="vertical" size="middle" style={{ width: "100%" }}>
          <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
            <div
              style={{
                height: 48,
                width: 48,
                borderRadius: 12,
                background: "linear-gradient(135deg, #1677ff 0%, #0050b3 100%)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "#ffffff",
                fontWeight: "extrabold",
                fontSize: 20,
                boxShadow: "0 4px 12px rgba(22, 119, 255, 0.3)",
              }}
            >
              Y
            </div>
            <div>
              <Title level={5} style={{ margin: 0 }}>
                YSN 截图翻译 (YSN Translator)
              </Title>
              <Space style={{ marginTop: 4 }}>
                <Text type="secondary" style={{ fontSize: 11 }}>
                  Tauri 2 + High-Performance Desktop Interface
                </Text>
                <Tag color="blue" style={{ fontSize: 10, margin: 0, borderRadius: 4 }}>
                  v1.0.0
                </Tag>
              </Space>
            </div>
          </div>

          <Paragraph style={{ fontSize: 12, color: "#595959", lineHeight: "1.8", margin: 0 }}>
            这是一款专为个人和工作室量身定制的高速截图翻译客户端。它完全剥离了传统 Electron 的沉重包袱，采用 
            <Text strong> Tauri 2.0 + React 19 + Ant Design v5</Text> 架构。前端借助 
            Vite 编译器直接驱动 Windows 本地 WebView2 容器进行高性能渲染，极大地缩减了内存开销（保持在约 15MB~30MB），并具有极佳的点击响应速度。
          </Paragraph>

          <Row gutter={16} style={{ marginTop: 12 }}>
            <Col span={12}>
              <Card type="inner" title={<span><ThunderboltOutlined style={{ marginRight: 6, color: "#1677ff" }} />N100 边缘算法服务器</span>}>
                <Paragraph type="secondary" style={{ fontSize: 11, margin: 0, lineHeight: 1.6 }}>
                  翻译与 OCR 处理内核部署于轻量级私有服务器，支持 PaddleOCR 物理像素提取以及 PIL 蒙版排版重绘，将 CPU/GPU 算力完美隔离。
                </Paragraph>
              </Card>
            </Col>
            <Col span={12}>
              <Card type="inner" title={<span><SafetyCertificateOutlined style={{ marginRight: 6, color: "#52c41a" }} />混合 OCR 协同调度</span>}>
                <Paragraph type="secondary" style={{ fontSize: 11, margin: 0, lineHeight: 1.6 }}>
                  支持 PaddleOCR 本地可执行程序极速运行及在线 API 自动回退，抵御高负荷并发场景，确保极端条件下的翻译率达 100%。
                </Paragraph>
              </Card>
            </Col>
          </Row>
        </Space>
      </Card>

      {/* Credit and Details Card */}
      <Card 
        title={<span><HeartOutlined style={{ marginRight: 8, color: "#ff4d4f" }} />鸣谢与开源社区</span>} 
        bordered={false}
        style={{ borderRadius: 12, boxShadow: "0 1px 3px rgba(0,0,0,0.02)" }}
      >
        <Space direction="vertical" size="middle" style={{ width: "100%" }}>
          <Paragraph style={{ fontSize: 12, color: "#595959", margin: 0 }}>
            该项目秉持自由开放的精神。在此鸣谢以下著名的开源技术贡献：
          </Paragraph>

          <div style={{ background: "#fafafa", borderRadius: 8, padding: "12px 16px" }}>
            {creditsList.map((item, index) => (
              <div 
                key={index} 
                style={{ 
                  display: "flex", 
                  justifyContent: "between", 
                  alignItems: "center", 
                  padding: "6px 0",
                  borderBottom: index === creditsList.length - 1 ? 0 : "1px solid #f0f0f0"
                }}
              >
                <Text style={{ fontFamily: "monospace", fontSize: 11, color: "#1f1f1f" }}>
                  {item.name}
                </Text>
                <Text type="secondary" style={{ fontSize: 10 }}>
                  {item.desc}
                </Text>
              </div>
            ))}
          </div>

          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", borderTop: "1px solid #f0f0f0", paddingTop: 16, fontSize: 11 }}>
            <Text type="secondary">
              © 2026 Developer you. Licensed under MIT.
            </Text>
            <Button
              type="link"
              icon={<GithubOutlined />}
              href="https://github.com/yoousn/screenshot-translator"
              target="_blank"
              style={{ fontSize: 12, padding: 0 }}
            >
              GitHub 仓库
            </Button>
          </div>
        </Space>
      </Card>
    </Space>
  );
}
