import React from "react";

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  description?: string;
}

export function Switch({ checked, onChange, label, description }: SwitchProps) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-slate-100 last:border-0">
      {label && (
        <div className="flex flex-col space-y-0.5 pr-4 select-none">
          <span className="text-xs font-semibold text-slate-700">{label}</span>
          {description && <span className="text-3xs text-slate-400 font-medium leading-normal">{description}</span>}
        </div>
      )}
      <button
        type="button"
        onClick={() => onChange(!checked)}
        className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-250 ease-in-out focus:outline-none focus:ring-2 focus:ring-primary-500/25 active:scale-90 transform ${
          checked ? "bg-primary-600" : "bg-slate-200"
        }`}
      >
        <span
          className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition-transform duration-250 ease-in-out ${
            checked ? "translate-x-4" : "translate-x-0"
          }`}
        />
      </button>
    </div>
  );
}
