import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "outline" | "ghost" | "destructive";
  size?: "sm" | "md" | "lg" | "icon";
  isLoading?: boolean;
}

export function Button({
  children,
  variant = "primary",
  size = "md",
  isLoading = false,
  className = "",
  disabled,
  ...props
}: ButtonProps) {
  const baseStyle =
    "inline-flex items-center justify-center font-medium transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-primary-500/20 active:scale-[0.98] select-none cursor-pointer disabled:opacity-50 disabled:pointer-events-none disabled:active:scale-100";

  const variants = {
    primary:
      "bg-primary-600 hover:bg-primary-700 text-white shadow-premium-sm hover:shadow-premium-md",
    secondary: "bg-slate-100 hover:bg-slate-200 text-slate-700 border border-slate-200/50",
    outline: "bg-white border border-slate-200 text-slate-600 hover:bg-slate-50 hover:text-slate-800",
    ghost: "bg-transparent text-slate-500 hover:bg-slate-100 hover:text-slate-700",
    destructive: "bg-rose-600 hover:bg-rose-700 text-white shadow-premium-sm",
  };

  const sizes = {
    sm: "px-3 py-1.5 text-xs rounded-md gap-1.5",
    md: "px-4 py-2 text-xs rounded-lg gap-2",
    lg: "px-5 py-2.5 text-sm rounded-lg gap-2",
    icon: "h-8 w-8 rounded-lg",
  };

  return (
    <button
      disabled={disabled || isLoading}
      className={`${baseStyle} ${variants[variant]} ${sizes[size]} ${className}`}
      {...props}
    >
      {isLoading ? (
        <>
          <svg
            className="animate-spin h-3.5 w-3.5 text-current"
            fill="none"
            viewBox="0 0 24 24"
          >
            <circle
              className="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              strokeWidth="4"
            />
            <path
              className="opacity-75"
              fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
            />
          </svg>
          {size !== "icon" && children}
        </>
      ) : (
        children
      )}
    </button>
  );
}
