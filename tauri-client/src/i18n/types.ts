export type AppLanguage = "zh-CN" | "en-US";

export type LanguageOption = {
  value: AppLanguage;
  label: string;
  shortLabel: string;
};

export type AppDictionary = {
  app: {
    name: string;
    tagline: string;
    screenshotNow: string;
    startScreenshot: string;
    startingScreenshot: string;
    screenshotStarted: string;
    screenshotFailed: string;
    refresh: string;
    language: string;
    shortcutErrorTitle: string;
    shortcutErrorDesc: string;
  };
  nav: {
    dashboard: string;
    settings: string;
    ocrConfig: string;
    history: string;
    about: string;
  };
  status: {
    service: string;
    online: string;
    offline: string;
    checking: string;
  };
  config: Record<string, string>;
  settings: Record<string, string>;
  ocrResult: Record<string, string>;
  dashboard: {
    title: string;
    desc: string;
    primaryAction: string;
    commandCenter: string;
    run: string;
    screenshot: string;
    screenshotDesc: string;
    translate: string;
    translateDesc: string;
    delayed: string;
    delayedDesc: string;
    ocr: string;
    ocrDesc: string;
    fullscreen: string;
    fullscreenDesc: string;
    startTranslateFailed: string;
    delayedInfo: string;
    delayedCancel: string;
    delayedStarting: string;
    fullscreenCopied: string;
    fullscreenFailed: string;
    hotkey: string;
    ocrMode: string;
    ocrModeValue: string;
    targetLang: string;
    targetLangDefault: string;
    service: string;
    sourceLanguage: string;
    sourceLanguageAuto: string;
    commercialReady: string;
    commercialReadyDesc: string;
    installModels: string;
    configureService: string;
    fixHotkey: string;
    allCoreReady: string;
    actionNeeded: string;
    diagnosticsTitle: string;
    diagnosticsDesc: string;
    diagnosticsRefresh: string;
    diagnosticsReady: string;
    diagnosticsNotReady: string;
    diagnosticsOpenRecovery: string;
    diagnosticsNoIssues: string;
    diagnosticsIssuesCount: string;
    diagnosticsModuleOcrRuntime: string;
    diagnosticsModuleRecording: string;
  };
};

export type I18nContextValue = {
  language: AppLanguage;
  text: AppDictionary;
  antdLocale: any;
  setLanguage: (language: AppLanguage) => Promise<void>;
};
