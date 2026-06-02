import React from "react";
import { Alert, Descriptions, Select, Space, Tag, Typography } from "antd";
import { TranslationOutlined } from "@ant-design/icons";
import ConfigSectionCard from "./ConfigSectionCard";
import type { TranslationLanguagePanelProps } from "./types";
import { useI18n } from "../../i18n";

const { Text } = Typography;

const targetLanguages = [
  { value: "zh", label: "简体中文" },
  { value: "zh-TW", label: "繁體中文" },
  { value: "en", label: "English" },
  { value: "ja", label: "日本語" },
  { value: "ko", label: "한국어" },
  { value: "fr", label: "Français" },
  { value: "de", label: "Deutsch" },
  { value: "es", label: "Español" },
  { value: "pt", label: "Português" },
  { value: "it", label: "Italiano" },
  { value: "ru", label: "Русский" },
  { value: "ar", label: "العربية" },
  { value: "th", label: "ไทย" },
  { value: "tr", label: "Türkçe" },
];

export default function TranslationLanguagePanel({ targetLang, onTargetLangChange }: TranslationLanguagePanelProps) {
  const { text } = useI18n();
  const labels = text.config;

  return (
    <ConfigSectionCard
      eyebrow={labels.translationEyebrow}
      title={<span><TranslationOutlined style={{ marginRight: 8 }} />{labels.translationTitle}</span>}
      description={labels.translationDesc}
    >
      <Alert type="info" showIcon message={labels.sourceLanguageAutoMessage} description={labels.sourceLanguageAutoDesc} />
      <Space direction="vertical" size={8} style={{ width: "100%" }}>
        <Text strong>{labels.targetLanguage}</Text>
        <Select style={{ width: "100%" }} value={targetLang || "zh"} options={targetLanguages} onChange={onTargetLangChange} />
      </Space>
      <Descriptions size="small" column={1} bordered>
        <Descriptions.Item label={labels.sourceLanguage}><Tag color="blue">{labels.sourceLanguageAuto}</Tag></Descriptions.Item>
        <Descriptions.Item label={labels.defaultTarget}><Tag color="green">简体中文</Tag></Descriptions.Item>
        <Descriptions.Item label={labels.supportedSourceScripts}>Latin, CJK, Kana, Hangul, Cyrillic, Arabic, Thai, Turkish, future model packs</Descriptions.Item>
        <Descriptions.Item label={labels.technicalTextPolicy}>{labels.technicalTextPolicyValue}</Descriptions.Item>
      </Descriptions>
    </ConfigSectionCard>
  );
}
