// GPUIX component definitions and native motion wrappers.

import { createElement, forwardRef, useCallback, useLayoutEffect, useState } from "react"
import type { ReactElement, ReactNode } from "react"
import type { EventPayload } from "@regenrek/gpuix-native"
import type {
  MotionProps,
  Props,
  PublicInstance,
  StyleDesc,
  VirtualListProps,
} from "../types/host.js"

export const gpuixComponents = {
  div: "div",
  text: "text",
  img: "img",
  svg: "svg",
  canvas: "canvas",
  input: "input",
  textarea: "textarea",
  anchored: "anchored",
  "virtual-list": "virtual-list",
  "split-view": "split-view",
  "dock-workspace": "dock-workspace",
  terminal: "terminal",
} as const

export type GpuixComponentType = keyof typeof gpuixComponents

export { SplitView } from "./split-view.js"
export type { SplitViewProps } from "./split-view.js"
export { DockWorkspace } from "./dock-workspace.js"
export type { DockLayout, DockPanel, DockWorkspaceProps } from "./dock-workspace.js"

export interface MotionDivProps extends MotionProps {
  children?: ReactNode
  style?: StyleDesc
  onClick?: Props["onClick"]
  onMouseDown?: Props["onMouseDown"]
  onMouseUp?: Props["onMouseUp"]
  onMouseEnter?: Props["onMouseEnter"]
  onMouseLeave?: Props["onMouseLeave"]
  onMouseMove?: Props["onMouseMove"]
  onMouseDownOutside?: Props["onMouseDownOutside"]
  onKeyDown?: Props["onKeyDown"]
  onKeyUp?: Props["onKeyUp"]
  onFocus?: Props["onFocus"]
  onBlur?: Props["onBlur"]
  onScroll?: Props["onScroll"]
  autoFocus?: boolean
}

const MotionDiv = forwardRef<PublicInstance, MotionDivProps>(function MotionDiv(
  { initial, animate, transition, ...props },
  ref
): ReactElement {
  const hostProps: Props = {
    ...props,
    ref,
    motion: {
      ...(initial === undefined ? {} : { initial }),
      animate,
      ...(transition === undefined ? {} : { transition }),
    },
  }
  return createElement("div", hostProps)
})

/** Native animations with a Motion-like declarative React API. */
export const motion = {
  div: MotionDiv,
} as const

export interface WindowedVirtualListProps extends VirtualListProps {
  itemCount: number
  renderItem: (index: number) => ReactNode
}

function initialWindow(options: {
  itemCount: number
  pad: number
  alignment: VirtualListProps["alignment"]
  followTail: boolean | undefined
}): { start: number; end: number } {
  if (options.followTail || options.alignment === "bottom") {
    return { start: Math.max(0, options.itemCount - options.pad), end: options.itemCount }
  }
  return { start: 0, end: Math.min(options.itemCount, options.pad) }
}

/** Mounts only the visible window of a virtual list. */
export const VirtualList = forwardRef<PublicInstance, WindowedVirtualListProps>(
  function VirtualList(
    {
      itemCount,
      renderItem,
      estimatedItemHeight = 48,
      overdraw = 240,
      alignment,
      followTail,
      onVisibleRange,
      children: _children,
      ...props
    },
    ref,
  ): ReactElement {
    const pad = Math.max(2, Math.ceil((800 + overdraw * 2) / Math.max(1, estimatedItemHeight)))
    const [range, setRange] = useState(() =>
      initialWindow({ itemCount, pad, alignment, followTail }),
    )
    useLayoutEffect(() => {
      setRange((current) => {
        if (followTail) {
          const next = initialWindow({ itemCount, pad, alignment, followTail })
          return current.start === next.start && current.end === next.end ? current : next
        }
        if (current.start >= itemCount) {
          const next = { start: Math.max(0, itemCount - pad), end: itemCount }
          return current.start === next.start && current.end === next.end ? current : next
        }
        const next = { start: current.start, end: Math.min(current.end, itemCount) }
        return current.start === next.start && current.end === next.end ? current : next
      })
    }, [alignment, followTail, itemCount, pad])
    const handleRange = useCallback(
      (event: EventPayload & { startIndex?: number | null; endIndex?: number | null }) => {
        const next = {
          start: Math.max(0, Math.floor(event.startIndex ?? 0) - pad),
          end: Math.min(itemCount, Math.ceil(event.endIndex ?? 0) + pad),
        }
        setRange((current) =>
          current.start === next.start && current.end === next.end ? current : next,
        )
        onVisibleRange?.(event)
      },
      [itemCount, onVisibleRange, pad],
    )
    const start = Math.min(range.start, itemCount)
    const end = Math.min(range.end, itemCount)
    return createElement(
      "virtual-list",
      {
        ...props,
        ref,
        alignment,
        followTail,
        estimatedItemHeight,
        overdraw,
        itemCount,
        windowStart: start,
        onVisibleRange: handleRange,
      },
      Array.from({ length: Math.max(0, end - start) }, (_, offset) =>
        renderItem(start + offset),
      ),
    )
  },
)
