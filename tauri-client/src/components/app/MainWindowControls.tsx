import { useEffect, useRef, useState } from "react";
import { CloseOutlined, BorderOutlined, FullscreenExitOutlined, MinusOutlined } from "@ant-design/icons";
import { getCurrentWindow } from "@tauri-apps/api/window";

type AppWindow = ReturnType<typeof getCurrentWindow>;

export default function MainWindowControls() {
  const windowRef = useRef<AppWindow | null>(null);
  const [maximized, setMaximized] = useState(false);

  const getWindow = () => {
    if (windowRef.current) return windowRef.current;
    try {
      windowRef.current = getCurrentWindow();
    } catch {
      return null;
    }
    return windowRef.current;
  };

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    const win = getWindow();
    if (!win) return;

    const syncMaximized = () => {
      win.isMaximized().then(setMaximized).catch(() => setMaximized(false));
    };

    syncMaximized();
    win.onResized(() => syncMaximized()).then((dispose) => {
      unlisten = dispose;
    }).catch(() => {});

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const minimize = () => {
    getWindow()?.minimize().catch(() => {});
  };

  const toggleMaximize = async () => {
    const win = getWindow();
    if (!win) return;
    await win.toggleMaximize().catch(() => {});
    win.isMaximized().then(setMaximized).catch(() => {});
  };

  const close = () => {
    getWindow()?.close().catch(() => {});
  };

  return (
    <div className="main-window-controls" data-no-drag="true">
      <button className="main-window-control" type="button" aria-label="Minimize window" onClick={minimize}>
        <MinusOutlined />
      </button>
      <button className="main-window-control" type="button" aria-label={maximized ? "Restore window" : "Maximize window"} onClick={toggleMaximize}>
        {maximized ? <FullscreenExitOutlined /> : <BorderOutlined />}
      </button>
      <button className="main-window-control main-window-control-close" type="button" aria-label="Close window" onClick={close}>
        <CloseOutlined />
      </button>
    </div>
  );
}
