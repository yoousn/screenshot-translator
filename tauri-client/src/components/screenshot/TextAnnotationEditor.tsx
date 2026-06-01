import React from "react";
import { Button, Input } from "antd";
import type { EditingTextDraft } from "../../types/screenshot";

interface TextAnnotationEditorProps {
  draft: NonNullable<EditingTextDraft>;
  onChange: (value: string) => void;
  onCommit: () => void;
  onCancel: () => void;
}

export default function TextAnnotationEditor({ draft, onChange, onCommit, onCancel }: TextAnnotationEditorProps) {
  const left = Math.max(8, Math.min(draft.x, window.innerWidth - 240));
  const top = Math.max(8, Math.min(draft.y, window.innerHeight - 48));

  return (
    <div
      style={{ position: "absolute", left, top, zIndex: 80, display: "flex", gap: 6, alignItems: "center", padding: 6, borderRadius: 8, background: "rgba(255,255,255,0.96)", boxShadow: "0 8px 24px rgba(0,0,0,0.16)" }}
      onMouseDown={(event) => event.stopPropagation()}
    >
      <Input
        autoFocus
        size="small"
        value={draft.value}
        placeholder="输入文字"
        style={{ width: 170 }}
        onChange={(event) => onChange(event.target.value)}
        onPressEnter={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onCommit();
        }}
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Escape") {
            event.preventDefault();
            onCancel();
          }
        }}
      />
      <Button size="small" type="primary" onClick={onCommit}>确定</Button>
    </div>
  );
}
