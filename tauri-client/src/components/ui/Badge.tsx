import React from "react";

interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: "success" | "error" | "warning" | "info" | "neutral";
}

export function Badge({ children, variant = "neutral", className = "", ...props }: BadgeProps) {
  const styles = {
    success: "bg-emerald-50 text-emerald-700 border-emerald-200/60",
    error: "bg-rose-50 text-rose-700 border-rose-200/60",
    warning: "bg-amber-50 text-amber-700 border-amber-200/60",
    info: "bg-primary-50 text-primary-700 border-primary-200/60",
    neutral: "bg-slate-50 text-slate-600 border-slate-200/80",
  };

  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-3xs font-semibold border tracking-wide select-none ${styles[variant]} ${className}`}
      {...props}
    >
      {children}
    </span>
  );
}
