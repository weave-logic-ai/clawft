import type { CanvasElementData } from "../../stores/canvas-store";
import { Button } from "../ui/button";

interface ButtonElementProps {
  element: CanvasElementData;
  onInteraction: (action: string) => void;
}

export function ButtonElement({ element, onInteraction }: ButtonElementProps) {
  const label = (element.label as string) ?? "Button";
  const action = (element.action as string) ?? "";
  const disabled = (element.disabled as boolean) ?? false;

  return (
    <Button
      variant="secondary"
      disabled={disabled}
      onClick={() => onInteraction(action)}
    >
      {label}
    </Button>
  );
}
