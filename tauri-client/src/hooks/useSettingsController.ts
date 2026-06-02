import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "antd";
import type { FormInstance } from "antd";
import type {
  ServerChannelStatus,
  TranslationChannel,
  TranslationChannelTestStatuses,
} from "../components/settings/types";

type ServerChannelPayload = {
  channel: string;
  config: Record<string, string>;
};

const trimTrailingSlash = (value: string) => value.replace(/\/$/, "");

const buildServerUrlCandidates = (values: any) => {
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
        throw new Error((data as any).error || `状态码：${response.status}`);
      }
      return { serverUrl, data, response };
    } catch (error: any) {
      errors.push(`${serverUrl}: ${error?.message || error}`);
    }
  }
  throw new Error(errors.join("; "));
};

export default function useSettingsController(form: FormInstance, onConfigSaved: () => void) {
  const [isSaving, setIsSaving] = useState(false);
  const [isTestingBaidu, setIsTestingBaidu] = useState(false);
  const [isTestingNewApi, setIsTestingNewApi] = useState(false);
  const [isFetchingModels, setIsFetchingModels] = useState(false);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [currentChannel, setCurrentChannel] = useState<string>("google");
  const [channelTestStatuses, setChannelTestStatuses] = useState<TranslationChannelTestStatuses>({});
  const [serverChannelStatus, setServerChannelStatus] = useState<ServerChannelStatus>({});

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const configStr = await invoke<string>("get_config");
      const parsedConfig = JSON.parse(configStr || "{}");

      form.setFieldsValue(parsedConfig);
      if (parsedConfig.channel) {
        setCurrentChannel(parsedConfig.channel);
      }

      const autostartEnabled = await invoke<boolean>("is_autostart_enabled");
      form.setFieldValue("autostart", autostartEnabled);

      if (parsedConfig.newApiBase && parsedConfig.newApiKey) {
        setAvailableModels([parsedConfig.newApiModel || "gemini-3.5-flash"]);
      }

      const serverUrls = buildServerUrlCandidates(parsedConfig);
      if (serverUrls.length > 0) {
        await syncActiveServerChannel(serverUrls, parsedConfig.clientToken || "");
      }
    } catch (error) {
      console.error(error);
      message.error("加载设置失败，请检查本地配置文件是否损坏。");
    }
  };

  const syncActiveServerChannel = async (serverUrls: string[], clientToken: string) => {
    try {
      const { serverUrl, data: serverConfig } = await requestJsonFromCandidates<any>(serverUrls, "/api/config/current", {
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
        error: error instanceof Error ? error.message : String(error),
        checkedAt: new Date().toISOString(),
      });
    }
  };

  const handleFormChange = (changedValues: Record<string, unknown>) => {
    if (changedValues.channel) {
      setCurrentChannel(String(changedValues.channel));
    }
  };

  const fetchModels = async () => {
    const values = form.getFieldsValue(true);
    const serverUrls = buildServerUrlCandidates(values);
    const clientToken = form.getFieldValue("clientToken") || "";
    const newApiBase = form.getFieldValue("newApiBase");
    const newApiKey = form.getFieldValue("newApiKey");

    if (serverUrls.length === 0) {
      message.error("请先填写并保存文本翻译服务地址。");
      return;
    }
    if (!newApiBase || !newApiKey) {
      message.error("请先填写 New API 中转地址和 API Key。");
      return;
    }

    setIsFetchingModels(true);
    try {
      const { data: resData } = await requestJsonFromCandidates<any>(serverUrls, "/api/config/fetch_models", {
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
        if (resData.models.length > 0 && !resData.models.includes(form.getFieldValue("newApiModel"))) {
          form.setFieldValue("newApiModel", resData.models[0]);
        }
      } else {
        throw new Error(resData.error || "模型列表拉取失败");
      }
    } catch (error: any) {
      message.error(`获取模型列表失败：${error.message || error}`);
    } finally {
      setIsFetchingModels(false);
    }
  };

  const testChannel = async (channel: TranslationChannel) => {
    const values = form.getFieldsValue(true);
    const serverUrls = buildServerUrlCandidates(values);
    const clientToken = form.getFieldValue("clientToken") || "";

    if (serverUrls.length === 0) {
      message.error("请先填写文本翻译服务地址。");
      return;
    }

    const testPayload: ServerChannelPayload = { channel, config: {} };

    if (channel === "baidu") {
      setIsTestingBaidu(true);
      testPayload.config = {
        app_id: form.getFieldValue("baiduAppId") || "",
        secret_key: form.getFieldValue("baiduSecretKey") || "",
      };
    } else {
      setIsTestingNewApi(true);
      testPayload.config = {
        base_url: form.getFieldValue("newApiBase") || "",
        api_key: form.getFieldValue("newApiKey") || "",
        model: form.getFieldValue("newApiModel") || "",
      };
    }

    setChannelTestStatuses((prev) => ({
      ...prev,
      [channel]: { status: "testing", testedAt: new Date().toISOString() },
    }));

    try {
      const { serverUrl, data: resData } = await requestJsonFromCandidates<any>(serverUrls, "/api/config/test", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "x-api-key": clientToken,
        },
        body: JSON.stringify(testPayload),
      });

      if (resData.status === "success") {
        const channelName = channel === "baidu" ? "百度翻译" : "大模型翻译";
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
      } else {
        throw new Error(resData.error || "接口验证失败");
      }
    } catch (error: any) {
      const errorMessage = error.message || String(error);
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
    }
  };

  const buildServerChannelPayload = (values: any): ServerChannelPayload => {
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
      };
    }
    return payload;
  };

  const saveServerChannelConfig = async (values: any) => {
    const serverUrls = buildServerUrlCandidates(values);
    const clientToken = values.clientToken || "";
    if (serverUrls.length === 0) {
      throw new Error("请先填写文本翻译服务地址。");
    }

    const { serverUrl, data: resData } = await requestJsonFromCandidates<any>(serverUrls, "/api/config/save", {
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

  const onFinish = async (values: any) => {
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
        });
      } catch (shortcutError: any) {
        message.warning(`本地配置已保存，但快捷键注册失败：${shortcutError.message || shortcutError}`);
      }
      await invoke("set_autostart_enabled", { enabled: Boolean(autostartVal) });

      let serverSaved = false;
      try {
        const savedServerUrl = await saveServerChannelConfig(configValues);
        serverSaved = true;
        setServerChannelStatus({
          activeChannel: configValues.channel || "google",
          serviceUrl: savedServerUrl,
          checkedAt: new Date().toISOString(),
        });
      } catch (serverError: any) {
        setServerChannelStatus({
          error: serverError.message || String(serverError),
          checkedAt: new Date().toISOString(),
        });
        message.warning(`本地设置已保存，但服务端翻译配置未同步：${serverError.message || serverError}`);
      }

      message.success(serverSaved ? "设置保存成功。" : "本地设置已保存。");
      onConfigSaved();
    } catch (error: any) {
      message.error(`保存失败：${error.message || error}`);
    } finally {
      setIsSaving(false);
    }
  };

  const restoreDefaultHotkeys = () => {
    form.setFieldsValue({ hotkey: "Alt+A", translateHotkey: "Alt+T" });
    message.success("已还原默认快捷键。");
  };

  return {
    isSaving,
    isTestingBaidu,
    isTestingNewApi,
    isFetchingModels,
    availableModels,
    currentChannel,
    channelTestStatuses,
    serverChannelStatus,
    handleFormChange,
    fetchModels,
    testChannel,
    onFinish,
    restoreDefaultHotkeys,
  };
}
