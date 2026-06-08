import { GithubOutlined, MailOutlined, TagOutlined, UserOutlined } from "@ant-design/icons";
import { Button, Space, Typography } from "antd";
import { openUrl } from "@tauri-apps/plugin-opener";
import packageInfo from "../../package.json";

const { Paragraph, Title } = Typography;

const REPOSITORY_URL = "https://github.com/yoousn/screenshot-translator";
const CONTACT_EMAIL = "gg1761229856@gmail.com";
const AUTHOR = "犹少年";

export default function About() {
  const openRepository = () => openUrl(REPOSITORY_URL).catch(() => {});
  const openEmail = () => openUrl(`mailto:${CONTACT_EMAIL}`).catch(() => {});

  return (
    <div className="about-center-page">
      <section className="about-hero-panel" aria-label="关于 截图翻译">
        <div className="about-hero-kicker">Ysn Trans</div>
        <Title level={2} className="about-hero-title">截图翻译</Title>
        <div className="about-hero-subtitle">Screenshot Translator</div>
        <Paragraph className="about-hero-description">轻量、快捷的桌面截图翻译工具，面向日常阅读、工作沟通和跨语言资料处理。</Paragraph>

        <div className="about-meta-list" aria-label="产品信息">
          <div className="about-meta-item"><TagOutlined /><span>版本</span><strong>v{packageInfo.version}</strong></div>
          <div className="about-meta-item"><UserOutlined /><span>作者</span><strong>{AUTHOR}</strong></div>
          <div className="about-meta-item"><MailOutlined /><span>联系</span><strong>{CONTACT_EMAIL}</strong></div>
        </div>

        <Space className="about-actions" wrap>
          <Button type="primary" icon={<GithubOutlined />} onClick={openRepository}>GitHub 仓库</Button>
          <Button icon={<MailOutlined />} onClick={openEmail}>联系作者</Button>
        </Space>
      </section>
    </div>
  );
}
