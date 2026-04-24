import type React from "react";

export function signedValueClass(value: number) {
  if (value > 0) {
    return "text-emerald-600 dark:text-emerald-400";
  }
  if (value < 0) {
    return "text-red-600 dark:text-red-400";
  }
  return "app-text-muted";
}

export function SignedValue({
  value,
  children,
  className = "",
}: {
  value: number;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <span className={`${signedValueClass(value)} ${className}`.trim()}>
      {children}
    </span>
  );
}
