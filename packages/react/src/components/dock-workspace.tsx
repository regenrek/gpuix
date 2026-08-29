import { createElement, Fragment } from "react"
import type { ReactElement, ReactNode } from "react"
import type { EventPayload } from "@regenrek/gpuix-native"
import type { StyleDesc } from "../types/host.js"

/** A normalized, serializable generic workbench tree. Panel and node IDs are
 * caller-owned strings, so unrelated React edits never change layout identity. */
export type DockLayout = DockTabs | DockSplit

export interface DockTabs {
  kind: "tabs"
  id: string
  panels: string[]
  active?: string
  zoomed?: string
}

export interface DockSplit {
  kind: "split"
  id: string
  direction: "horizontal" | "vertical"
  ratio: number
  first: DockLayout
  second: DockLayout
  zoomed?: string
}

export interface DockPanel {
  id: string
  label: string
  content: ReactNode
  closable?: boolean
}

export interface DockWorkspaceProps {
  /** Controlled tree. Native code normalizes all committed mutations. */
  layout: DockLayout
  /** Generic panel descriptors; content identity follows the stable string ID. */
  panels: readonly DockPanel[]
  style?: StyleDesc
  testId?: string
  /** Receives one committed normalized tree after a native tab/drop action. */
  onLayoutChange?: (layout: DockLayout) => void
  /** Requests native keyboard focus for a stable panel ID. */
  focusPanelId?: string
  /** Receives keys from the native workbench focus surface. */
  onKeyDown?: (event: EventPayload) => void
  tabIndex?: number
  autoFocus?: boolean
  accessibilityName?: string
}

/**
 * A controlled native docking surface. Pointer movement, drag preview, drop
 * hit-testing, and geometry are entirely native; React receives only an
 * accepted semantic layout mutation.
 */
export function DockWorkspace({
  layout,
  panels,
  onLayoutChange,
  focusPanelId,
  accessibilityName = "Workbench",
  ...props
}: DockWorkspaceProps): ReactElement {
  const panelIds = panels.map((panel) => panel.id)
  const labels = Object.fromEntries(panels.map((panel) => [panel.id, panel.label]))
  const closable = Object.fromEntries(panels.map((panel) => [panel.id, panel.closable === true]))
  return createElement(
    "dock-workspace",
    {
      ...props,
      layout,
      panelIds,
      labels,
      closable,
      focusPanelId,
      tabIndex: props.tabIndex ?? 0,
      accessibilityRole: "group",
      accessibilityName,
      onLayoutChange: (event: EventPayload & { layout?: string | null }) => {
        if (!event.layout) return
        onLayoutChange?.(JSON.parse(event.layout) as DockLayout)
      },
    },
    panels.map((panel) => createElement(Fragment, { key: panel.id }, panel.content)),
  )
}
