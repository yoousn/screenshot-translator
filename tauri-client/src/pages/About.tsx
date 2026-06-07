import { Typography } from "antd";

const { Paragraph, Title } = Typography;

export default function About() {
  return (
    <div className="about-center-page">
      <section className="about-hero-panel" aria-label="关于 截图翻译">
        <Title level={1} className="about-hero-title">
          截图翻译
        </Title>
        <div className="about-hero-subtitle">Screenshot Translator</div>
        <Paragraph className="about-hero-description">
          轻量、快捷的桌面截图翻译工具，面向日常阅读、工作沟通和跨语言资料处理。
        </Paragraph>
      </section>
    </div>
  );
}
