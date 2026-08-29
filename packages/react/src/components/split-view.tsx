import { Children, createElement } from "react"
import type { ReactElement, ReactNode } from "react"
import type { EventPayload } from "@regenrek/gpuix-native"
import type { StyleDesc } from "../types/host.js"

export interface SplitViewProps {
  children: ReactNode
  /** Split left/right (`horizontal`) or top/bottom (`vertical`). */
  direction?: "horizontal" | "vertical"
  /** Controlled fraction assigned to the first pane. */
  ratio?: number
  /** Initial fraction assigned to the first pane when `ratio` is omitted. */
  defaultRatio?: number
  /** Minimum native size of the first pane, in pixels. */
  minSize?: number
  /** Minimum native size of the second pane, in pixels. */
  minSecondSize?: number
  /** Native divider and drag-target thickness, in pixels. */
  dividerSize?: number
  style?: StyleDesc
  testId?: string
  /** Called once on a completed native drag; cancellation emits nothing. */
  onResize?: (ratio: number) => void
}

/** A two-pane native split view. Pointer moves stay in GPUI; React sees one commit. */
export function SplitView({
  children, direction = "horizontal", ratio, defaultRatio = 0.5, minSize = 0,
  minSecondSize = 0, dividerSize = 6, onResize, ...props
}: SplitViewProps): ReactElement {
  if (Children.count(children) !== 2) throw new Error("SplitView requires exactly two children")
  return createElement("split-view", {
    ...props, direction, ratio, defaultRatio, minSize, minSecondSize, dividerSize,
    onResize: (event: EventPayload) => onResize?.(event.ratio ?? defaultRatio),
  }, children)
}
