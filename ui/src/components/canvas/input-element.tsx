import { useState } from "react";
import type { CanvasElementData } from "../../stores/canvas-store";
import { Button } from "../ui/button";

interface InputElementProps {
  element: CanvasElementData;
  onSubmit: (value: string) => void;
}

export function InputElement({ element, onSubmit }: InputElementProps) {
  const label = (element.label as string) ?? "";
  const placeholder = (element.placeholder as string) ?? "";
  const defaultValue = (element.value as string) ?? "";
  const [value, setValue] = useState(defaultValue);

  const handleSubmit = () => {
    onSubmit(value);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSubmit();
    }
  };

  return (
    <div className="space-y-1">
      {label && (
        <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
          {label}
        </label>
      )}
      <div className="flex gap-2">
        <input
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          className="flex-1 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
        />
        <Button variant="secondary" size="sm" onClick={handleSubmit}>
          Submit
        </Button>
      </div>
    </div>
  );
}
