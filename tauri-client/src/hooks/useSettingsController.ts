import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { App as AntdApp } from "antd";
import type { FormInstance } from "antd";
import type {
  ServerChannelStatus,
  TranslationChannel,
  TranslationChannelTestStatuses,
} from "../components/settings/types";
import type { Config } from "../types/config";
import { DEFAULT_LLM_TRANSLATION_DOMAIN, DEFAULT_LLM_TRANSLATION_PROMPT } from "../utils/defaultTranslationPrompt";

type ServerChannelPayload = {
  channel: string;
  config: Record<string, string>;
};

type SettingsFormValues = Config & {
  autostart?: boolean;
  baiduAppId?: string;
  baiduSecretKey?: string;
  newApiBase?: string;
  newApiKey?: string;
  newApiModel?: string;
  newApiPrompt?: string;
  newApiDomain?: string;
  deeplEndpoint?: string;
  deeplApiKey?: string;
  deeplFormality?: string;
};

type ServerCurrentConfigResponse = {
  status?: string;
  active_channel?: string;
  error?: string;
};

type ModelListResponse = {
  status?: string;
  models?: string[];
  error?: string;
};

type ChannelTestResponse = {
  status?: string;
  result?: string;
  error?: string;
};

type ConfigSaveResponse = {
  status?: string;
  error?: string;
};

type SaveSettingsOptions = {
  showMessage?: boolean;
  successMessage?: string;
  syncServer?: boolean;
};

const trimTrailingSlash = (value: string) => value.replace(/\/$/, "");
const DEFAULT_MODEL = "gemini-2.0-flash";
const LEGACY_DEFAULT_MODEL = "gemini-3.5-flash";
const DEFAULT_HOTKEYS = {
  hotkey: "Alt+A",
  translateHotkey: "Alt+T",
  recordingHotkey: "Alt+R",
};
const PRIVATE_TRANSLATION_ERROR_PATTERN =
  /https?:\/\/|x-api-key|client[_\s-]*token|ocr\.yousn\.me|serverUrl|lanServerUrl|\b(?:\d{1,3}\.){3}\d{1,3}\b/i;

const publicTranslationServiceError = (error: unknown) => {
  const messageText = error instanceof Error ? error.message : String(error);
  if (PRIVATE_TRANSLATION_ERROR_PATTERN.test(messageText)) {
    return "翻译服务暂不可用，请稍后重试。";
  }
  return messageText;
};

const errorMessage = (error: unknown) => (error instanceof Error ? error.message : String(error));

const buildServerUrlCandidates = (values: Pick<SettingsFormValues, "serverUrl" | "lanServerUrl" | "preferLanServer">) => {
  const remoteUrl = values.serverUrl || "";
  const candidates = [
    ...(values.preferLanServer && values.lanServerUrl ? [values.lanServerUrl] : []),
    remoteUrl,
  ];
  return Array.from(new Set(candidates.map((item) => String(item || "").trim()).filter(Boolean)));
};

type CandidateJsonResult<T> = {
  serverUrl: string;
  data: T;
  response: Response;
};

const requestJsonFromCandidates = async <T>(
  serverUrls: string[],
  path: string,
  init: RequestInit,
): Promise<CandidateJsonResult<T>> => {
  const errors: string[] = [];
  for (const serverUrl of serverUrls) {
    try {
      const response = await fetch(`${trimTrailingSlash(serverUrl)}${path}`, init);
      const data = await response.json().catch(() => ({} as T));
      if (!response.ok) {
        const responseError =
          typeof data === "object" && data !== null && "error" in data
            ? String((data as { error?: unknown }).error || "")
            : "";
        throw new Error(responseError || `状态码：${response.status}`);
      }
      return { serverUrl, data, response };
    } catch (error: unknown) {
      errors.push(`${serverUrl}: ${errorMessage(error)}`);
    }
  }
  console.warn("Translation service request failed", { path, errors });
  throw new Error("翻译服务暂不可用，请稍后重试。");
};

export default function useSettingsController(form: FormInstance, onConfigSaved: () => void) {
  const { message } = AntdApp.useApp();
  const [isSaving, setIsSaving] = useState(false);
  const [isActivatingGoogle, setIsActivatingGoogle] = useState(false);
  const [isTestingBaidu, setIsTestingBaidu] = useState(false);
  const [isTestingNewApi, setIsTestingNewApi] = useState(false);
  const [isTestingDeepl, setIsTestingDeepl] = useState(false);
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [currentChannel, setCurrentChannel] = useState<string>("google");
  const [channelTestStatuses, setChannelTestStatuses] = useState<TranslationChannelTestStatuses>({});
  const [serverChannelStatus, setServerChannelStatus] = useState<ServerChannelStatus>({});
  const autoSaveTimerRef = useRef<ReturnType<typeof window.setTimeout> | null>(null);
  const isLoadingSettingsRef = useRef(false);

  useEffect(() => {
    loadSettings();
    return () => {
      if (autoSaveTimerRef.current !== null) {
        window.clearTimeout(autoSaveTimerRef.current);
      }
    };
  }, []);

  const loadSettings = async () => {
    isLoadingSettingsRef.current = true;
    try {
      const configStr = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(configStr || "{}") as SettingsFormValues;
      const normalizedConfig = {
        ...parsedConfig,
        imageSaveNameFormat: !parsedConfig.imageSaveNameFormat || parsedConfig.imageSaveNameFormat === "yyyyMMdd_HHmm"
          ? "yyyyMMdd_HHmmss"
          : parsedConfig.imageSaveNameFormat,
      };

      form.setFieldsValue({
        ...normalizedConfig,
        newApiPrompt: normalizedConfig.newApiPrompt || DEFAULT_LLM_TRANSLATION_PROMPT,
        newApiDomain: normalizedConfig.newApiDomain || DEFAULT_LLM_TRANSLATION_DOMAIN,
      });
      if (parsedConfig.channel) {
        setCurrentChannel(parsedConfig.channel);
      }

      const autostartEnabled = await invoke<boolean>("is_autostart_enabled");
      form.setFieldValue("autostart", autostartEnabled);

      if (parsedConfig.newApiBase && parsedConfig.newApiKey) {
        setAvailableModels([parsedConfig.newApiModel || DEFAULT_MODEL]);
      }

      const serverUrls = buildServerUrlCandidates(parsedConfig);
      if (serverUrls.length > 0) {
        await syncActiveServerChannel(serverUrls, parsedConfig.clientToken || "");
      }
    } catch (error) {
      console.error(error);
      message.error("加载设置失败，请检查本地配置文件是否损坏。");
    } finally {
      isLoadingSettingsRef.current = false;
    }
  };

  const syncActiveServerChannel = async (serverUrls: string[], clientToken: string) => {
    try {
      const { serverUrl, data: serverConfig } = await requestJsonFromCandidates<ServerCurrentConfigResponse>(serverUrls, "/api/config/current", {
        headers: { "x-api-key": clientToken },
      });
      if (serverConfig.status === "success" && serverConfig.active_channel) {
        setCurrentChannel(serverConfig.active_channel);
        form.setFieldValue("channel", serverConfig.active_channel);
        setServerChannelStatus({
          activeChannel: serverConfig.active_channel,
          serviceUrl: serverUrl,
          checkedAt: new Date().toISOString(),
        });
      } else {
        throw new Error(serverConfig.error || "服务端未返回当前翻译通道。");
      }
    } catch (error) {
      console.warn("Failed to sync server active channel", error);
      setServerChannelStatus({
        error: publicTranslationServiceError(error),
        checkedAt: new Date().toISOString(),
      });
    }
  };

  const handleFormChange = (changedValues: Record<string, unknown>) => {
    if (changedValues.channel) {
      setCurrentChannel(String(changedValues.channel));
    }
    if (isLoadingSettingsRef.current) return;
    if (autoSaveTimerRef.current !== null) {
      window.clearTimeout(autoSaveTimerRef.current);
    }
    autoSaveTimerRef.current = window.setTimeout(() => {
      autoSaveTimerRef.current = null;
      void saveSettingsValues(form.getFieldsValue(true) as SettingsFormValues, { showMessage: true, successMessage: "已自动保存", syncServer: false });
    }, 400);
  };

  const fetchModels = async () => {
    const values = form.getFieldsValue(true) as SettingsFormValues;
    const serverUrls = buildServerUrlCandidates(values);
    const clientToken = form.getFieldValue("clientToken") || "";
    const newApiBase = form.getFieldValue("newApiBase");
    const newApiKey = form.getFieldValue("newApiKey");

    if (serverUrls.length === 0) {
      message.error("翻译服务未配置，请联系维护者。");
      return;
    }
    if (!newApiBase || !newApiKey) {
      message.error("请先填写大模型中转地址和 API Key。");
      return;
    }

    setIsFetchingModels(true);
    try {
      const { data: resData } = await requestJsonFromCandidates<ModelListResponse>(serverUrls, "/api/config/fetch_models", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": clientToken,
        },
        body: JSON.stringify({
          base_url: newApiBase,
          api_key: newApiKey,
        }),
      });

      if (resData.status === "success" && Array.isArray(resData.models)) {
        setAvailableModels(resData.models);
        message.success(`模型列表拉取成功，共 ${resData.models.length} 个模型。`);
        const currentModel = String(form.getFieldValue("newApiModel") || "").trim();
        const shouldAdoptFirstModel = !currentModel || (currentModel === LEGACY_DEFAULT_MODEL && !resData.models.includes(currentModel));
        if (resData.models.length > 0 && shouldAdoptFirstModel) {
          form.setFieldValue("newApiModel", resData.models[0]);
        }
      } else {
        throw new Error(resData.error || "模型列表拉取失败");
      }
    } catch (error: unknown) {
      message.warning(`获取模型列表失败，不影响手动填写模型：${publicTranslationServiceError(error)}`);
    } finally {
      setIsFetchingModels(false);
    }
  };

  const testChannel = async (channel: TranslationChannel) => {
    const values = form.getFieldsValue(true) as SettingsFormValues;
    const serverUrls = buildServerUrlCandidates(values);
    const clientToken = form.getFieldValue("clientToken") || "";

    if (serverUrls.length === 0) {
      message.error("翻译服务未配置，请联系维护者。");
      return;
    }

    const testPayload: ServerChannelPayload = { channel, config: {} };

    if (channel === "baidu") {
      setIsTestingBaidu(true);
      testPayload.config = {
        app_id: form.getFieldValue("baiduAppId") || "",
        secret_key: form.getFieldValue("baiduSecretKey") || "",
      };
    } else if (channel === "new-api") {
      setIsTestingNewApi(true);
      testPayload.config = {
        base_url: form.getFieldValue("newApiBase") || "",
        api_key: form.getFieldValue("newApiKey") || "",
        model: form.getFieldValue("newApiModel") || "",
        prompt: form.getFieldValue("newApiPrompt") || DEFAULT_LLM_TRANSLATION_PROMPT,
        domain: form.getFieldValue("newApiDomain") || DEFAULT_LLM_TRANSLATION_DOMAIN,
      };
    } else {
      setIsTestingDeepl(true);
      testPayload.config = {
        endpoint: form.getFieldValue("deeplEndpoint") || "https://api-free.deepl.com",
        api_key: form.getFieldValue("deeplApiKey") || "",
        formality: form.getFieldValue("deeplFormality") || "default",
      };
    }

    setChannelTestStatuses((prev) => ({
      ...prev,
      [channel]: { status: "testing", testedAt: new Date().toISOString() },
    }));

    try {
      const { serverUrl, data: resData } = await requestJsonFromCandidates<ChannelTestResponse>(serverUrls, "/api/config/test", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": clientToken,
        },
        body: JSON.stringify(testPayload),
      });

      if (resData.status === "success") {
        const channelName = channel === "baidu" ? "百度翻译" : channel === "deepl" ? "DeepL 翻译" : "大模型翻译";
        message.success(`翻译通道「${channelName}」测试通过，并已设为当前活动通道。`);
        form.setFieldValue("channel", channel);
        setCurrentChannel(channel);
        setChannelTestStatuses((prev) => ({
          ...prev,
          [channel]: {
            status: "passed",
            message: resData.result,
            serviceUrl: serverUrl,
            testedAt: new Date().toISOString(),
          },
        }));
        setServerChannelStatus({
          activeChannel: channel,
          serviceUrl: serverUrl,
          checkedAt: new Date().toISOString(),
        });
        onConfigSaved();
      } else {
        throw new Error(resData.error || "接口验证失败");
      }
    } catch (error: unknown) {
      const errorMessage = publicTranslationServiceError(error);
      setChannelTestStatuses((prev) => ({
        ...prev,
        [channel]: {
          status: "failed",
          message: errorMessage,
          testedAt: new Date().toISOString(),
        },
      }));
      message.error(`测试连接失败：${errorMessage}`);
    } finally {
      setIsTestingBaidu(false);
      setIsTestingNewApi(false);
      setIsTestingDeepl(false);
    }
  };

  const buildServerChannelPayload = (values: SettingsFormValues): ServerChannelPayload => {
    const channel = values.channel || "google";
    const payload: ServerChannelPayload = { channel, config: {} };
    if (channel === "baidu") {
      payload.config = {
        app_id: values.baiduAppId || "",
        secret_key: values.baiduSecretKey || "",
      };
    } else if (channel === "new-api") {
      payload.config = {
        base_url: values.newApiBase || "",
        api_key: values.newApiKey || "",
        model: values.newApiModel || "",
        prompt: values.newApiPrompt || DEFAULT_LLM_TRANSLATION_PROMPT,
        domain: values.newApiDomain || DEFAULT_LLM_TRANSLATION_DOMAIN,
      };
    } else if (channel === "deepl") {
      payload.config = {
        endpoint: values.deeplEndpoint || "https://api-free.deepl.com",
        api_key: values.deeplApiKey || "",
        formality: values.deeplFormality || "default",
      };
    }
    return payload;
  };

  const saveServerChannelConfig = async (values: SettingsFormValues) => {
    const serverUrls = buildServerUrlCandidates(values);
    const clientToken = values.clientToken || "";
    if (serverUrls.length === 0) {
      throw new Error("翻译服务未配置，请联系维护者。");
    }

    const { serverUrl, data: resData } = await requestJsonFromCandidates<ConfigSaveResponse>(serverUrls, "/api/config/save", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-api-key": clientToken,
      },
      body: JSON.stringify(buildServerChannelPayload(values)),
    });
    if (resData.status !== "success") {
      throw new Error(resData.error || "服务端配置保存失败。");
    }
    return serverUrl;
  };

  const saveLocalChannelOnly = async (channel: string) => {
    let existingConfig = {};
    try {
      existingConfig = JSON.parse(await invoke<string>("get_config"));
    } catch {}
    await invoke("save_config", {
      configStr: JSON.stringify({ ...existingConfig, channel }, null, 4),
    });
  };

  const saveSettingsValues = async (values: SettingsFormValues, options: SaveSettingsOptions = {}) => {
    const { showMessage = true, successMessage, syncServer = false } = options;
    setIsSaving(true);
    try {
      const { autostart: autostartVal, ...rawConfigValues } = values;
      let existingConfig = {};
      try {
        existingConfig = JSON.parse(await invoke<string>("get_config"));
      } catch {}
      const configValues = {
        ...existingConfig,
        ...rawConfigValues,
        useLocalOcr: true,
        fallbackToRemoteOcr: false,
      };
      const configStr = JSON.stringify(configValues, null, 4);
      await invoke("save_config", { configStr });
      try {
        await invoke("re_register_shortcut", {
          hotkey: configValues.hotkey || "",
          translateHotkey: configValues.translateHotkey || "",
          recordingHotkey: configValues.recordingHotkey || "",
        });
      } catch (shortcutError: unknown) {
        message.warning(`本地配置已保存，但快捷键注册失败：${errorMessage(shortcutError)}`);
      }
      await invoke("set_autostart_enabled", { enabled: Boolean(autostartVal) });

      let serverSaved = false;
      if (syncServer) {
        try {
          const savedServerUrl = await saveServerChannelConfig(configValues);
          serverSaved = true;
          setServerChannelStatus({
            activeChannel: configValues.channel || "google",
            serviceUrl: savedServerUrl,
            checkedAt: new Date().toISOString(),
          });
        } catch (serverError: unknown) {
          const errorMessage = publicTranslationServiceError(serverError);
          setServerChannelStatus({
            error: errorMessage,
            checkedAt: new Date().toISOString(),
          });
          message.warning(`本地设置已保存，但服务端翻译配置未同步：${errorMessage}`);
        }
      }

      if (showMessage) {
        message.success(successMessage || (syncServer && serverSaved ? "设置保存成功。" : "本地设置已保存。"));
      }
      onConfigSaved();
    } catch (error: unknown) {
      message.error(`保存失败：${errorMessage(error)}`);
    } finally {
      setIsSaving(false);
    }
  };

  const activateGoogleChannel = async () => {
    const values = { ...(form.getFieldsValue(true) as SettingsFormValues), channel: "google" };
    form.setFieldValue("channel", "google");
    setCurrentChannel("google");
    setIsActivatingGoogle(true);
    try {
      const savedServerUrl = await saveServerChannelConfig(values);
      await saveLocalChannelOnly("google");
      setServerChannelStatus({
        activeChannel: "google",
        serviceUrl: savedServerUrl,
        checkedAt: new Date().toISOString(),
      });
      message.success("Google Translate 已设为当前活动通道。");
      onConfigSaved();
    } catch (error: unknown) {
      const errorMessage = publicTranslationServiceError(error);
      setServerChannelStatus({
        error: errorMessage,
        checkedAt: new Date().toISOString(),
      });
      message.error(`Google 通道启用失败：${errorMessage}`);
    } finally {
      setIsActivatingGoogle(false);
    }
  };

  const onFinish = async (values: SettingsFormValues) => {
    await saveSettingsValues(values, { showMessage: true, syncServer: true });
  };

  const applyHotkeyPatch = (patch: Record<string, string>, successMessage: string) => {
    if (autoSaveTimerRef.current !== null) {
      window.clearTimeout(autoSaveTimerRef.current);
      autoSaveTimerRef.current = null;
    }
    form.setFieldsValue(patch);
    void saveSettingsValues({ ...(form.getFieldsValue(true) as SettingsFormValues), ...patch }, {
      showMessage: true,
      successMessage,
      syncServer: false,
    });
  };

  const updateHotkeyValue = (field: "hotkey" | "translateHotkey" | "recordingHotkey", value: string) => {
    applyHotkeyPatch({ [field]: value }, "快捷键已保存并生效。");
  };

  const clearScreenshotHotkey = () => {
    applyHotkeyPatch({ hotkey: "" }, "截图快捷键已清空并生效。");
  };

  const clearTranslateHotkey = () => {
    applyHotkeyPatch({ translateHotkey: "" }, "翻译快捷键已清空并生效。");
  };

  const clearRecordingHotkey = () => {
    applyHotkeyPatch({ recordingHotkey: "" }, "录制快捷键已清空并生效。");
  };

  const restoreDefaultHotkeys = () => {
    applyHotkeyPatch(DEFAULT_HOTKEYS, "已还原默认快捷键并生效。");
  };

  return {
    isSaving,
    isActivatingGoogle,
    isTestingBaidu,
    isTestingNewApi,
    isTestingDeepl,
    isFetchingModels,
    availableModels,
    currentChannel,
    channelTestStatuses,
    serverChannelStatus,
    handleFormChange,
    fetchModels,
    activateGoogleChannel,
    testChannel,
    onFinish,
    restoreDefaultHotkeys,
    updateHotkeyValue,
    clearScreenshotHotkey,
    clearTranslateHotkey,
    clearRecordingHotkey,
  };
}
