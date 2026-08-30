import { Focusable } from "@decky/ui";
import type { ReactNode } from "react";

export interface BackHandlerProps {
  onBack?: () => void;
  children: ReactNode;
}

export function BackHandler({ onBack, children }: BackHandlerProps) {
  return <Focusable onCancelButton={onBack}>{children}</Focusable>;
}
