import { describe, expect, it } from "vitest"
import React from "react"
import { SplitView } from "../components/index.js"
import { createTestRoot, hasNativeTestRenderer } from "../testing.js"

const describeNative = hasNativeTestRenderer ? describe : describe.skip

describeNative("SplitView", () => {
  it("captures a drag outside its handle, clamps it, and emits only its committed ratio", () => {
    const root = createTestRoot()
    const commits: number[] = []
    root.render(
      <SplitView
        defaultRatio={0.5}
        minSize={80}
        minSecondSize={100}
        dividerSize={6}
        onResize={(ratio) => commits.push(ratio)}
        style={{ width: 400, height: 120 }}
      >
        <div style={{ backgroundColor: "#111111" }} />
        <div style={{ backgroundColor: "#222222" }} />
      </SplitView>,
    )

    root.renderer.nativeSimulateMouseDown(200, 60)
    root.renderer.nativeSimulateMouseMove(390, 60, 0)
    root.renderer.nativeSimulateMouseMove(399, 60, 0)
    expect(commits).toEqual([])
    root.renderer.nativeSimulateMouseUp(399, 60)

    expect(commits).toHaveLength(1)
    // Available width is 394px; the second pane is clamped to its 100px minimum.
    expect(commits[0]).toBeCloseTo(294 / 394, 6)
  })

  it("keeps capture across repaint and cancels when the pointer leaves the split bounds", () => {
    const root = createTestRoot()
    const commits: number[] = []
    root.render(
      <SplitView defaultRatio={0.5} onResize={(ratio) => commits.push(ratio)} style={{ width: 400, height: 120 }}>
        <div />
        <div />
      </SplitView>,
    )

    root.renderer.nativeSimulateMouseDown(200, 60)
    // Mouse down flushes/repaints. This move proves capture was rebound to the
    // fresh native hitbox before deliberately leaving the whole split.
    root.renderer.nativeSimulateMouseMove(250, 60, 0)
    root.renderer.nativeSimulateMouseMove(401, 60, 0)
    root.renderer.nativeSimulateMouseUp(401, 60)

    expect(commits).toEqual([])
  })

  it("cancels an active drag when its bounds change", () => {
    const root = createTestRoot()
    const commits: number[] = []
    const view = (width: number) => (
      <SplitView defaultRatio={0.5} onResize={(ratio) => commits.push(ratio)} style={{ width, height: 120 }}>
        <div />
        <div />
      </SplitView>
    )

    root.render(view(400))
    root.renderer.nativeSimulateMouseDown(200, 60)
    root.render(view(500))
    root.renderer.nativeSimulateMouseUp(250, 60)

    expect(commits).toEqual([])
  })

  it("releases active native capture when unmounted", () => {
    const root = createTestRoot()
    const commits: number[] = []
    root.render(
      <SplitView onResize={(ratio) => commits.push(ratio)} style={{ width: 400, height: 120 }}>
        <div />
        <div />
      </SplitView>,
    )

    root.renderer.nativeSimulateMouseDown(200, 60)
    root.unmount()
    root.renderer.nativeSimulateMouseMove(250, 60, 0)
    root.renderer.nativeSimulateMouseUp(250, 60)

    expect(commits).toEqual([])
  })

  it("uses the vertical axis for geometry and final commit", () => {
    const root = createTestRoot()
    const commits: number[] = []
    root.render(
      <SplitView direction="vertical" defaultRatio={0.5} minSize={50} minSecondSize={60} onResize={(ratio) => commits.push(ratio)} style={{ width: 120, height: 300 }}>
        <div />
        <div />
      </SplitView>,
    )

    root.renderer.nativeSimulateMouseDown(60, 150)
    root.renderer.nativeSimulateMouseMove(60, 20, 0)
    root.renderer.nativeSimulateMouseUp(60, 20)

    expect(commits).toHaveLength(1)
    expect(commits[0]).toBeCloseTo(50 / 294, 6)
  })
})
