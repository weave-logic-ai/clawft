import type { CanvasElementData } from "../../stores/canvas-store";

interface TextElementProps {
  element: CanvasElementData;
}

export function TextElement({ element }: TextElementProps) {
  const content = (element.content as string) ?? "";
  const format = (element.format as string) ?? "plain";

  if (format === "markdown") {
    // For now, render markdown as a styled pre block.
    // A full markdown renderer can be added later.
    return (
      <div className="prose dark:prose-invert max-w-none whitespace-pre-wrap text-sm text-gray-800 dark:text-gray-200">
        {content}
      </div>
    );
  }

  return (
    <p className="text-sm text-gray-800 dark:text-gray-200">{content}</p>
  );
}
