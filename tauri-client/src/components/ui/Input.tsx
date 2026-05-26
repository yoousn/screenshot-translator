import React from "react";

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  icon?: React.ReactNode;
}

export function Input({ className = "", icon, type = "text", ...props }: InputProps) {
  return (
    <div className="relative w-full">
      {icon && (
        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-400">
          {icon}
        </div>
      )}
      <input
        type={type}
        className={`w-full text-xs border border-slate-200 hover:border-slate-300 focus:border-primary-500 focus:ring-2 focus:ring-primary-500/10 rounded-lg py-2 transition-all bg-slate-50/10 text-slate-700 placeholder:text-slate-400 ${
          icon ? "pl-9 pr-3" : "px-3"
        } ${className}`}
        {...props}
      />
    </div>
  );
}
