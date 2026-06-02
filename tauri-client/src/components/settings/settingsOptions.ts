export const getChannelOptions = (labels: Record<string, string>) => [
  { value: "google", label: labels.channelGoogle },
  { value: "baidu", label: labels.channelBaidu },
  { value: "new-api", label: labels.channelNewApi },
  { value: "deepl", label: labels.channelDeepL },
];

export const getTargetLangOptions = (labels: Record<string, string>) => [
  { value: "zh", label: labels.langZhHans },
  { value: "zh-TW", label: labels.langZhHant },
  { value: "en", label: labels.langEn },
  { value: "ja", label: labels.langJa },
  { value: "ko", label: labels.langKo },
  { value: "fr", label: labels.langFr },
  { value: "de", label: labels.langDe },
  { value: "es", label: labels.langEs },
  { value: "pt", label: labels.langPt },
  { value: "it", label: labels.langIt },
  { value: "ru", label: labels.langRu },
  { value: "ar", label: labels.langAr },
  { value: "th", label: labels.langTh },
  { value: "tr", label: labels.langTr },
];

export const hotkeyPattern = /^(|((Alt|Ctrl|Control|Shift|Cmd|Command|Meta|Win|Windows)(\s*\+\s*(Alt|Ctrl|Control|Shift|Cmd|Command|Meta|Win|Windows))*\s*\+\s*.+))$/i;
