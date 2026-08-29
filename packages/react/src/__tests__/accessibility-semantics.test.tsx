import React from "react"
import { describe, expect, it } from "vitest"
import { DockWorkspace, type DockLayout } from "../components/dock-workspace.js"
import { createTestRoot, hasNativeTestRenderer } from "../testing.js"

const describeNative = hasNativeTestRenderer ? describe : describe.skip

describeNative("accessibility semantics", () => {
  it("projects generic workbench semantics through the production retained automation tree", () => {
    const root = createTestRoot()
    const layout: DockLayout = { kind: "tabs", id: "main", panels: ["panel"] }
    root.render(
      <DockWorkspace
        layout={layout}
        panels={[{ id: "panel", label: "Panel", content: <div /> }]}
        accessibilityName="Workspace"
        testId="workbench"
        style={{ width: 300, height: 200 }}
      />,
    )
    const tree = JSON.parse(root.renderer.getAutomationTree()) as {
      accessibility?: Record<string, unknown>
    }
    expect(tree.accessibility).toEqual({ role: "group", name: "Workspace" })
  })

  it("forwards built-in element roles, names, values, and states to the same tree", () => {
    const root = createTestRoot()
    root.render(
      <div
        accessibilityRole="button"
        accessibilityName="Run"
        accessibilityValue="ready"
        accessibilityDisabled={false}
        accessibilityExpanded={true}
        accessibilitySelected={true}
        accessibilityChecked={false}
      />,
    )
    const tree = JSON.parse(root.renderer.getAutomationTree()) as {
      accessibility?: Record<string, unknown>
    }
    expect(tree.accessibility).toEqual({
      role: "button",
      name: "Run",
      value: "ready",
      disabled: false,
      expanded: true,
      selected: true,
      checked: false,
    })
  })
})
