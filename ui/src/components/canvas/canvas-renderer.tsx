import { useCanvasStore } from "../../stores/canvas-store";
import { CanvasElementSwitch } from "./canvas-element";
import { Card, CardContent } from "../ui/card";

interface CanvasRendererProps {
  onInteraction: (interaction: {
    type: string;
    element_id: string;
    [key: string]: unknown;
  }) => void;
}

export function CanvasRenderer({ onInteraction }: CanvasRendererProps) {
  const elements = useCanvasStore((state) => state.elements);
  const entries = Array.from(elements.values());

  if (entries.length === 0) {
    return (
      <div className="flex h-64 items-center justify-center">
        <div className="text-center">
          <p className="text-lg font-medium text-gray-500 dark:text-gray-400">
            Canvas is empty
          </p>
          <p className="mt-1 text-sm text-gray-400 dark:text-gray-500">
            Agents will render UI elements here when they use the render_ui tool.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {entries.map((element) => (
        <Card key={element.id}>
          <CardContent className="p-4">
            <CanvasElementSwitch
              element={element}
              onInteraction={onInteraction}
            />
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
