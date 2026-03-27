import type { CanvasElementData } from "../../stores/canvas-store";
import { TextElement } from "./text-element";
import { ButtonElement } from "./button-element";
import { InputElement } from "./input-element";
import { CodeElement, CodeEditorElement } from "./code-element";
import { TableElement } from "./table-element";
import { FormElement, AdvancedFormElement } from "./form-element";
import { ChartElement } from "./chart-element";

interface CanvasElementSwitchProps {
  element: CanvasElementData;
  onInteraction: (interaction: {
    type: string;
    element_id: string;
    [key: string]: unknown;
  }) => void;
}

export function CanvasElementSwitch({
  element,
  onInteraction,
}: CanvasElementSwitchProps) {
  switch (element.type) {
    case "text":
      return <TextElement element={element} />;

    case "button":
      return (
        <ButtonElement
          element={element}
          onInteraction={(action) =>
            onInteraction({
              type: "click",
              element_id: element.id,
              action,
            })
          }
        />
      );

    case "input":
      return (
        <InputElement
          element={element}
          onSubmit={(value) =>
            onInteraction({
              type: "input_submit",
              element_id: element.id,
              value,
            })
          }
        />
      );

    case "image":
      return (
        <img
          src={element.src as string}
          alt={(element.alt as string) ?? ""}
          className="max-w-full rounded-md"
        />
      );

    case "code":
      return <CodeElement element={element} />;

    case "code_editor":
      return (
        <CodeEditorElement
          element={element}
          onInteraction={onInteraction}
        />
      );

    case "table":
      return <TableElement element={element} />;

    case "form":
      return (
        <FormElement
          element={element}
          onSubmit={(values) =>
            onInteraction({
              type: "form_submit",
              element_id: element.id,
              values,
            })
          }
        />
      );

    case "form_advanced":
      return (
        <AdvancedFormElement
          element={element}
          onInteraction={onInteraction}
        />
      );

    case "chart":
      return <ChartElement element={element} />;

    default:
      return (
        <div className="rounded-md border border-yellow-300 bg-yellow-50 p-3 text-sm text-yellow-700 dark:border-yellow-600 dark:bg-yellow-900/20 dark:text-yellow-400">
          Unknown element type: {element.type}
        </div>
      );
  }
}
