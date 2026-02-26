import type { HTMLAttributes } from "react";
import { cn } from "../../lib/utils";

const variants = {
  default: "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300",
  secondary:
    "bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-300",
  destructive:
    "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300",
  outline:
    "border border-gray-300 text-gray-700 dark:border-gray-600 dark:text-gray-300",
  success:
    "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300",
} as const;

interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: keyof typeof variants;
}

export function Badge({ variant = "default", className, ...props }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium",
        variants[variant],
        className,
      )}
      {...props}
    />
  );
}
