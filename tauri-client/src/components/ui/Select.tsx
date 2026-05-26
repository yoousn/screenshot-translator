import React from "react";

interface Option {
  value: string;
  label: string;
}

interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  options: Option[];
}

export function Select({ options, className = "", ...props }: SelectProps) {
  return (
    <div className="relative w-full">
      <select
        className={`w-full text-xs border border-slate-200 hover:border-slate-300 focus:border-primary-500 focus:ring-2 focus:ring-primary-500/10 rounded-lg px-3 py-2 transition-all bg-white text-slate-700 appearance-none cursor-pointer pr-8 ${className}`}
        {...props}
      >
        {options.map((opt) => (
          <option key={opt.value} value={opt.value}>
            {opt.label}
          </option>
        ))}
      </select>
      <div className="absolute inset-y-0 right-0 flex items-center pr-2.5 pointer-events-none text-slate-400">
        <svg
          className="h-4 w-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="2"
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </div>
    </div>
  );
}
