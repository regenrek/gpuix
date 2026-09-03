import React, { useState } from "react"
import fs from "fs"
import { describe, expect, it } from "vitest"
import {
  DockWorkspace,
  type DockLayout,
  type DockPanel,
} from "../components/dock-workspace.js"
import { createTestRoot, hasNativeTestRenderer } from "../testing.js"
import type { GpuixTheme } from "../types/host.js"
import { expectScreenshotsDiffer, SHOTS_DIR } from "./test-utils.js"

const describeNative = hasNativeTestRenderer ? describe : describe.skip

const panels: DockPanel[] = [
  { id: "a", label: "A", content: <div testId="panel-a">panel a</div>, closable: true },
  { id: "b", label: "B", content: <div testId="panel-b">panel b</div>, closable: true },
  { id: "c", label: "C", content: <div testId="panel-c">panel c</div>, closable: true },
]

const customTheme: GpuixTheme = {
  bg: "#110011",
  border: "#ff00ff",
  text: "#ffffff",
  textMuted: "#dddddd",
  accent: "#aa00aa",
  metrics: {
    dockTabHeight: 28,
    dockTabPaddingX: 6,
    dockControlGap: 3,
    dockControlPaddingX: 3,
  },
}

function ControlledDock({
  initialLayout,
  onCommit,
  focusPanelId,
  onKeyDown,
}: {
  initialLayout: DockLayout
  onCommit: (layout: DockLayout) => void
  focusPanelId?: string
  onKeyDown?: () => void
}) {
  const [layout, setLayout] = useState(initialLayout)
  return (
    <DockWorkspace
      layout={layout}
      panels={panels}
      onLayoutChange={(next) => {
        onCommit(next)
        setLayout(next)
      }}
      focusPanelId={focusPanelId}
      onKeyDown={onKeyDown}
      style={{ width: 800, height: 400 }}
      testId="workbench"
    />
  )
}

function panelIds(layout: DockLayout): string[] {
  if (layout.kind === "tabs") return layout.panels
  return [...panelIds(layout.first), ...panelIds(layout.second)]
}

describeNative("DockWorkspace", () => {
  it("forwards one complete theme contract to native dock chrome", () => {
    const root = createTestRoot()
    const layout = { kind: "tabs", id: "root", panels: ["a"], active: "a" } satisfies DockLayout
    const defaultShot = `${SHOTS_DIR}/dock-default-theme.png`
    const customShot = `${SHOTS_DIR}/dock-custom-theme.png`
    if (fs.existsSync(defaultShot)) fs.unlinkSync(defaultShot)
    if (fs.existsSync(customShot)) fs.unlinkSync(customShot)

    root.render(<DockWorkspace layout={layout} panels={panels} style={{ width: 320, height: 180 }} testId="workbench" />)
    root.renderer.captureScreenshot(defaultShot)
    root.render(
      <DockWorkspace
        layout={layout}
        panels={panels}
        theme={customTheme}
        style={{ width: 320, height: 180 }}
        testId="workbench"
      />,
    )
    root.renderer.captureScreenshot(customShot)

    expect(root.renderer.findByTestId("workbench")?.customProps?.theme).toEqual(customTheme)
    expectScreenshotsDiffer(defaultShot, customShot)
  })

  it("keeps long multi-tab headers inside the native workbench width", () => {
    const root = createTestRoot()
    const commits: DockLayout[] = []
    const densePanels: DockPanel[] = ["a", "b", "c", "d", "e"].map((id) => ({
      id,
      label: `Long conversation title ${id.toUpperCase()}`,
      content: <div>{`panel ${id}`}</div>,
      closable: true,
    }))
    root.render(
      <DockWorkspace
        layout={{ kind: "tabs", id: "root", panels: ["a", "b", "c", "d", "e"], active: "a" }}
        panels={densePanels}
        onLayoutChange={(next) => commits.push(next)}
        style={{ width: 320, height: 180 }}
      />,
    )

    root.renderer.nativeSimulateClick(288, 14)

    expect(commits.at(-1)).toMatchObject({ kind: "tabs", id: "root", active: "e" })
    expect(root.renderer.getPaintedText()).toContain("panel e")
  })

  it("keeps active panel inputs pointer-interactive", () => {
    const root = createTestRoot()

    function InputPanel() {
      const [value, setValue] = useState("")
      return (
        <DockWorkspace
          layout={{ kind: "tabs", id: "root", panels: ["input"], active: "input" }}
          panels={[{
            id: "input",
            label: "Input",
            content: (
              <input
                testId="dock-input"
                value={value}
                onChange={(event) => setValue(event.value ?? "")}
                style={{ width: 300, height: 40 }}
              />
            ),
          }]}
          style={{ width: 800, height: 400 }}
        />
      )
    }

    root.render(<InputPanel />)
    root.renderer.nativeSimulateClick(150, 52)
    root.renderer.simulateKeystrokes("a")

    expect(root.renderer.findByTestId("dock-input")?.customProps?.value).toBe("a")
  })

  it("rehydrates and renders a typed persisted root zoom state through the controlled path", () => {
    const root = createTestRoot()
    const persistedLayout = {
      kind: "split",
      id: "root",
      direction: "horizontal",
      ratio: 0.5,
      first: { kind: "tabs", id: "left", panels: ["a"], active: "a" },
      second: { kind: "tabs", id: "right", panels: ["b", "c"], active: "b" },
      zoomed: "b",
    } satisfies DockLayout
    const rehydrated: DockLayout = JSON.parse(JSON.stringify(persistedLayout))

    root.render(<ControlledDock initialLayout={rehydrated} onCommit={() => undefined} />)

    expect(rehydrated).toMatchObject({ kind: "split", zoomed: "b" })
    expect(root.renderer.getAllText()).toEqual(expect.arrayContaining(["panel b"]))
  })

  it("uses current nested tab-group bounds for repeated native edge drops without losing panels", () => {
    const root = createTestRoot()
    const commits: DockLayout[] = []
    const layout: DockLayout = {
      kind: "tabs",
      id: "root",
      panels: ["a", "b", "c"],
      active: "a",
    }
    root.render(<ControlledDock initialLayout={layout} onCommit={(next) => commits.push(next)} />)

    root.renderer.nativeSimulateMouseDown(12, 15)
    root.renderer.nativeSimulateMouseMove(790, 180, 0)
    root.renderer.nativeSimulateMouseUp(790, 180)
    root.renderer.nativeSimulateMouseDown(12, 15)
    root.renderer.nativeSimulateMouseMove(390, 180, 0)
    root.renderer.nativeSimulateMouseUp(390, 180)

    expect(commits).toHaveLength(2)
    expect([...new Set(panelIds(commits.at(-1)!))].sort()).toEqual(["a", "b", "c"])
    expect(panelIds(commits.at(-1)!)).toHaveLength(3)
  })

  it("cancels split and stale-target drops without loss, then accepts a later captured drag", () => {
    const root = createTestRoot()
    const commits: DockLayout[] = []
    const layout: DockLayout = {
      kind: "split",
      id: "root",
      direction: "horizontal",
      ratio: 0.5,
      first: { kind: "tabs", id: "left", panels: ["a"], active: "a" },
      second: { kind: "tabs", id: "right", panels: ["b", "c"], active: "b" },
    }
    root.render(<ControlledDock initialLayout={layout} onCommit={(next) => commits.push(next)} />)

    const beforeDrag = `${SHOTS_DIR}/dock-before-drag.png`
    const ghostDrag = `${SHOTS_DIR}/dock-invalid-split-ghost.png`
    if (fs.existsSync(beforeDrag)) fs.unlinkSync(beforeDrag)
    if (fs.existsSync(ghostDrag)) fs.unlinkSync(ghostDrag)
    root.renderer.captureScreenshot(beforeDrag)
    root.renderer.nativeSimulateMouseDown(12, 15)
    root.renderer.nativeSimulateMouseMove(400, 180, 0)
    root.renderer.captureScreenshot(ghostDrag)
    expectScreenshotsDiffer(beforeDrag, ghostDrag)
    root.renderer.nativeSimulateMouseUp(400, 180)
    expect(commits).toHaveLength(0)
    expect(root.renderer.getAllText()).toEqual(expect.arrayContaining(["panel a", "panel b", "panel c"]))

    root.renderer.nativeSimulateMouseDown(12, 15)
    root.renderer.nativeSimulateMouseMove(900, 180, 0)
    root.renderer.nativeSimulateMouseUp(900, 180)
    expect(commits).toHaveLength(0)
    root.renderer.nativeSimulateMouseDown(12, 15)
    root.renderer.nativeSimulateMouseMove(410, 180, 0)
    root.renderer.nativeSimulateMouseUp(410, 180)
    expect(commits).toHaveLength(1)
    expect([...new Set(panelIds(commits[0]))].sort()).toEqual(["a", "b", "c"])
  })

  it("clears pre-rerender geometry before a captured drop can commit", () => {
    const root = createTestRoot()
    const commits: DockLayout[] = []
    const layout: DockLayout = {
      kind: "tabs",
      id: "root",
      panels: ["a", "b"],
      active: "a",
    }
    const render = (width: number) =>
      root.render(
        <DockWorkspace
          layout={layout}
          panels={panels}
          onLayoutChange={(next) => commits.push(next)}
          style={{ width, height: 400 }}
          testId="workbench"
        />,
      )

    render(800)
    root.renderer.nativeSimulateMouseDown(12, 15)
    render(400)
    root.renderer.nativeSimulateMouseUp(700, 180)

    expect(commits).toEqual([])
    expect(root.renderer.getAllText()).toEqual(expect.arrayContaining(["panel a", "panel b"]))
  })

  it("commits native resize, close/collapse, zoom, requested focus, and only changed layouts", () => {
    const root = createTestRoot()
    const commits: DockLayout[] = []
    const keys: string[] = []
    const layout: DockLayout = {
      kind: "split",
      id: "root",
      direction: "horizontal",
      ratio: 0.5,
      first: { kind: "tabs", id: "left", panels: ["a"], active: "a" },
      second: { kind: "tabs", id: "right", panels: ["b"], active: "b" },
    }
    root.render(
      <ControlledDock
        initialLayout={layout}
        focusPanelId="a"
        onCommit={(next) => commits.push(next)}
        onKeyDown={() => keys.push("key")}
      />,
    )

    root.renderer.nativeSimulateMouseDown(400, 180)
    root.renderer.nativeSimulateMouseMove(560, 180, 0)
    root.renderer.nativeSimulateMouseUp(560, 180)
    expect(commits).toHaveLength(1)
    expect(commits[0]).toMatchObject({ kind: "split", ratio: expect.any(Number) })

    root.renderer.simulateKeystrokes("x")
    expect(keys).toEqual(["key"])

    root.renderer.nativeSimulateMouseDown(12, 15)
    root.renderer.nativeSimulateMouseUp(12, 15)
    expect(commits).toHaveLength(1)

    root.renderer.nativeSimulateClick(765, 35)
    expect(commits).toHaveLength(2)
    expect(commits[1]).toMatchObject({ kind: "tabs", panels: ["a"] })

    root.renderer.nativeSimulateClick(784, 35)
    expect(commits).toHaveLength(3)
    expect(JSON.stringify(commits[2])).toContain('"zoomed":"a"')
  })

  it("resizes from the ergonomic hit area around a one-pixel divider", () => {
    const root = createTestRoot()
    const commits: DockLayout[] = []
    const layout: DockLayout = {
      kind: "split",
      id: "root",
      direction: "horizontal",
      ratio: 0.5,
      first: { kind: "tabs", id: "left", panels: ["a"], active: "a" },
      second: { kind: "tabs", id: "right", panels: ["b"], active: "b" },
    }
    root.render(<ControlledDock initialLayout={layout} onCommit={(next) => commits.push(next)} />)

    root.renderer.nativeSimulateMouseDown(398, 180)
    root.renderer.nativeSimulateMouseMove(560, 180, 0)
    root.renderer.nativeSimulateMouseUp(560, 180)

    expect(commits).toHaveLength(1)
    expect(commits[0]).toMatchObject({ kind: "split", ratio: expect.any(Number) })
  })
})
