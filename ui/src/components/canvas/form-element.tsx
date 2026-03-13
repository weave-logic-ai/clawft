import { useState, useCallback } from "react";
import type { CanvasElementData } from "../../stores/canvas-store";
import { Button } from "../ui/button";

interface FormFieldData {
  name: string;
  label: string;
  field_type?: string;
  required?: boolean;
  placeholder?: string;
}

interface FormElementProps {
  element: CanvasElementData;
  onSubmit: (values: Record<string, string>) => void;
}

/**
 * Basic form element for the "form" canvas element type.
 * Supports text input fields with required validation.
 */
export function FormElement({ element, onSubmit }: FormElementProps) {
  const fields = (element.fields as FormFieldData[]) ?? [];
  const submitAction = (element.submit_action as string) ?? "Submit";

  const [values, setValues] = useState<Record<string, string>>(() => {
    const initial: Record<string, string> = {};
    for (const field of fields) {
      initial[field.name] = "";
    }
    return initial;
  });

  const handleChange = useCallback((name: string, value: string) => {
    setValues((prev) => ({ ...prev, [name]: value }));
  }, []);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(values);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      {fields.map((field) => (
        <div key={field.name} className="space-y-1">
          <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
            {field.label}
            {field.required && (
              <span className="ml-1 text-red-500">*</span>
            )}
          </label>
          <input
            type={field.field_type ?? "text"}
            name={field.name}
            value={values[field.name] ?? ""}
            onChange={(e) => handleChange(field.name, e.target.value)}
            placeholder={field.placeholder ?? ""}
            required={field.required ?? false}
            className="w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500"
          />
        </div>
      ))}
      <Button type="submit" variant="default">
        {submitAction}
      </Button>
    </form>
  );
}

// ── Advanced Form Element (S3.2) ──────────────────────────────

interface AdvancedFormFieldData {
  name: string;
  type: string;
  label: string;
  required?: boolean;
  options?: string[];
  min?: number;
  max?: number;
  placeholder?: string;
}

interface AdvancedFormElementProps {
  element: CanvasElementData;
  onInteraction: (interaction: {
    type: string;
    element_id: string;
    [key: string]: unknown;
  }) => void;
}

interface FieldError {
  field: string;
  message: string;
}

const inputClasses =
  "w-full rounded-md border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 placeholder-gray-400 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 dark:placeholder-gray-500";

const errorClasses = "border-red-400 focus:border-red-500 focus:ring-red-500";

/**
 * Advanced form element for the "form_advanced" canvas element type.
 * Supports text, number, select, checkbox, and textarea field types
 * with basic validation (required fields, number range).
 */
export function AdvancedFormElement({
  element,
  onInteraction,
}: AdvancedFormElementProps) {
  const fields = (element.fields as AdvancedFormFieldData[]) ?? [];
  const submitAction = (element.submitAction as string) ?? "Submit";

  const [values, setValues] = useState<Record<string, string>>(() => {
    const initial: Record<string, string> = {};
    for (const field of fields) {
      initial[field.name] = field.type === "checkbox" ? "false" : "";
    }
    return initial;
  });

  const [errors, setErrors] = useState<FieldError[]>([]);

  const handleChange = useCallback((name: string, value: string) => {
    setValues((prev) => ({ ...prev, [name]: value }));
    // Clear error for this field on change
    setErrors((prev) => prev.filter((e) => e.field !== name));
  }, []);

  const validate = useCallback((): FieldError[] => {
    const fieldErrors: FieldError[] = [];

    for (const field of fields) {
      const value = values[field.name] ?? "";

      // Required check
      if (field.required) {
        if (field.type === "checkbox" && value !== "true") {
          fieldErrors.push({
            field: field.name,
            message: `${field.label} is required`,
          });
          continue;
        }
        if (field.type !== "checkbox" && value.trim() === "") {
          fieldErrors.push({
            field: field.name,
            message: `${field.label} is required`,
          });
          continue;
        }
      }

      // Number range check
      if (field.type === "number" && value.trim() !== "") {
        const num = Number(value);
        if (isNaN(num)) {
          fieldErrors.push({
            field: field.name,
            message: `${field.label} must be a number`,
          });
        } else {
          if (field.min !== undefined && num < field.min) {
            fieldErrors.push({
              field: field.name,
              message: `${field.label} must be at least ${field.min}`,
            });
          }
          if (field.max !== undefined && num > field.max) {
            fieldErrors.push({
              field: field.name,
              message: `${field.label} must be at most ${field.max}`,
            });
          }
        }
      }
    }

    return fieldErrors;
  }, [fields, values]);

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      const fieldErrors = validate();
      if (fieldErrors.length > 0) {
        setErrors(fieldErrors);
        return;
      }
      onInteraction({
        type: "form_submit",
        element_id: element.id,
        values,
      });
    },
    [validate, onInteraction, element.id, values],
  );

  const getFieldError = (name: string): string | undefined =>
    errors.find((e) => e.field === name)?.message;

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      {fields.map((field) => {
        const fieldError = getFieldError(field.name);
        const hasError = !!fieldError;

        return (
          <div key={field.name} className="space-y-1">
            {field.type !== "checkbox" && (
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                {field.label}
                {field.required && (
                  <span className="ml-1 text-red-500">*</span>
                )}
              </label>
            )}

            {/* Text input */}
            {field.type === "text" && (
              <input
                type="text"
                name={field.name}
                value={values[field.name] ?? ""}
                onChange={(e) => handleChange(field.name, e.target.value)}
                placeholder={field.placeholder ?? ""}
                className={`${inputClasses} ${hasError ? errorClasses : ""}`}
              />
            )}

            {/* Number input */}
            {field.type === "number" && (
              <input
                type="number"
                name={field.name}
                value={values[field.name] ?? ""}
                onChange={(e) => handleChange(field.name, e.target.value)}
                placeholder={field.placeholder ?? ""}
                min={field.min}
                max={field.max}
                className={`${inputClasses} ${hasError ? errorClasses : ""}`}
              />
            )}

            {/* Select */}
            {field.type === "select" && (
              <select
                name={field.name}
                value={values[field.name] ?? ""}
                onChange={(e) => handleChange(field.name, e.target.value)}
                className={`${inputClasses} ${hasError ? errorClasses : ""}`}
              >
                <option value="">
                  {field.placeholder ?? "Select..."}
                </option>
                {(field.options ?? []).map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
              </select>
            )}

            {/* Checkbox */}
            {field.type === "checkbox" && (
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  name={field.name}
                  checked={values[field.name] === "true"}
                  onChange={(e) =>
                    handleChange(
                      field.name,
                      e.target.checked ? "true" : "false",
                    )
                  }
                  className={`h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 dark:border-gray-600 ${hasError ? "border-red-400" : ""}`}
                />
                <span className="text-sm text-gray-700 dark:text-gray-300">
                  {field.label}
                  {field.required && (
                    <span className="ml-1 text-red-500">*</span>
                  )}
                </span>
              </label>
            )}

            {/* Textarea */}
            {field.type === "textarea" && (
              <textarea
                name={field.name}
                value={values[field.name] ?? ""}
                onChange={(e) => handleChange(field.name, e.target.value)}
                placeholder={field.placeholder ?? ""}
                rows={4}
                className={`${inputClasses} resize-y ${hasError ? errorClasses : ""}`}
              />
            )}

            {/* Error message */}
            {hasError && (
              <p className="text-xs text-red-500">{fieldError}</p>
            )}
          </div>
        );
      })}

      <Button type="submit" variant="default">
        {submitAction}
      </Button>
    </form>
  );
}
