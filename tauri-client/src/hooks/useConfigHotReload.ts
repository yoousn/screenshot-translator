import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * 监听后端广播的配置变更事件（`config-changed`），在配置被写入磁盘后
 * 通知当前窗口重新拉取配置，使功能开关 / 模型版本 / 翻译目标语等
 * 无需重启即可在下一次行为中生效（工单②）。
 *
 * 设计要点：
 * - Rust `save_config` / `set_config_value_if_changed` 写盘成功后会 emit 该事件，
 *   所有 webview 窗口（截图后台窗口、主设置窗口等）都会收到。
 * - 收到事件后只调用回调（通常是 `loadConfig`），由回调负责刷新内存 configRef / state。
 *   这比在事件 payload 里直接塞整份配置更稳：始终以 `get_config` 读到的最新值为准，
 *   避免 payload 与磁盘缓存出现不一致的边角情况。
 * - 回调用 ref 持有，避免每次渲染都重新订阅。
 * - 通过 `excludeSelfSource` 可忽略自己发起的变更（可选）。
 *
 * @param onChanged 配置变更回调（通常为 `loadConfig`）
 * @param options.excludeSelfSource 若提供，则忽略 payload.source === 该值 的事件
 */
export function useConfigHotReload(
  onChanged: () => void | Promise<void>,
  options?: {
    excludeSelfSource?: string;
    shouldDefer?: () => boolean;
  },
) {
  const callbackRef = useRef(onChanged);
  callbackRef.current = onChanged;
  const excludeSourceRef = useRef(options?.excludeSelfSource);
  excludeSourceRef.current = options?.excludeSelfSource;
  const shouldDeferRef = useRef(options?.shouldDefer);
  shouldDeferRef.current = options?.shouldDefer;
  const pendingRef = useRef(false);
  const runningRef = useRef(false);
  const mountedRef = useRef(false);

  const requestReload = () => {
    if (!mountedRef.current) return;
    if (shouldDeferRef.current?.() || runningRef.current) {
      pendingRef.current = true;
      return;
    }

    pendingRef.current = false;
    runningRef.current = true;
    void Promise.resolve()
      .then(() => callbackRef.current())
      .catch((error) => {
        console.warn("[useConfigHotReload] onChanged handler failed", error);
      })
      .finally(() => {
        runningRef.current = false;
        if (!mountedRef.current || !pendingRef.current) return;
        if (shouldDeferRef.current?.()) return;
        queueMicrotask(requestReload);
      });
  };

  useEffect(() => {
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;
    mountedRef.current = true;

    listen<{ config?: string; source?: string }>("config-changed", (event) => {
      const payload = event.payload || {};
      const exclude = excludeSourceRef.current;
      if (exclude && payload.source === exclude) return;
      requestReload();
    })
      .then((unsub) => {
        if (cancelled) {
          unsub();
          return;
        }
        unlisten = unsub;
      })
      .catch((error) => {
        console.warn("[useConfigHotReload] failed to subscribe config-changed", error);
      });

    return () => {
      cancelled = true;
      mountedRef.current = false;
      pendingRef.current = false;
      if (unlisten) unlisten();
    };
  }, []);

  // A deferred event is flushed after the consumer's busy state causes a render.
  useEffect(() => {
    if (pendingRef.current && !shouldDeferRef.current?.()) {
      requestReload();
    }
  });
}
