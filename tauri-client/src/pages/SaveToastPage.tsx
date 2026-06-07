import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

export default function SaveToastPage() {
  useEffect(() => {
    const timer = window.setTimeout(() => {
      getCurrentWindow().close().catch(() => {});
    }, 1600);
    return () => window.clearTimeout(timer);
  }, []);

  return (
    <div className="save-toast-shell">
      <span className="save-toast-check">✓</span>
      <span className="save-toast-title">保存成功</span>
    </div>
  );
}
