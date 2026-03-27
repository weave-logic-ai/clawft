import { type InputHTMLAttributes } from 'react';

interface Props extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
}

export function FormField({ label, id, ...rest }: Props) {
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="text-xs text-gray-400">
        {label}
      </label>
      <input
        id={id}
        className="bg-gray-800 border border-gray-600 rounded px-3 py-1.5 text-sm text-gray-100
                   focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500/50
                   placeholder:text-gray-500"
        {...rest}
      />
    </div>
  );
}
