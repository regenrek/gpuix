import { describe, expect, it } from "vitest"
import React from "react"
import { SplitView } from "../components/index.js"
import { createTestRoot, hasNativeTestRenderer } from "../testing.js"

const describeNative = hasNativeTestRenderer ? describe : describe.skip

function rect(root: ReturnType<typeof createTestRoot>, testId: string) {
  const element = root.renderer.findByTestId(testId)
  expect(element, `missing ${testId}`).toBeDefined()
  const bounds = root.renderer.getElementBounds(element!.id)
  expect(bounds, `missing bounds for ${testId}`).toEqual(expect.any(Array))
  return { x: bounds![0], y: bounds![1], width: bounds![2], height: bounds![3] }
}

describeNative("SplitView", () => {
  it.each([
    ["horizontal", { width: 400, height: 120 }, "width", 200, 60, 390, 60],
    ["vertical", { width: 120, height: 400 }, "height", 60, 200, 60, 390],
  ] as const)("moves %s panes natively during a drag and emits one final event", (direction, style, axis, downX, downY, endX, endY) => {
    const root = createTestRoot()
    const commits: number[] = []
    root.render(
      <SplitView
        direction={direction}
        defaultRatio={0.5}
        minSize={80}
        minSecondSize={100}
        dividerSize={6}
        onResize={(ratio) => commits.push(ratio)}
        style={style}
      >
        <div testId="first" style={{ flexGrow: 1, backgroundColor: "#111111" }} />
        <div testId="second" style={{ flexGrow: 1, backgroundColor: "#222222" }} />
      </SplitView>,
    )

    expect(rect(root, "first")[axis]).toBeCloseTo(197, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo(197, 6)

    root.renderer.nativeSimulateMouseDown(downX, downY)
    root.renderer.nativeSimulateMouseMove(endX, endY, 0)
    expect(commits).toEqual([])
    // Both panes move in the native frame before React receives the final
    // semantic event on mouse-up.
    expect(rect(root, "first")[axis]).toBeCloseTo(294, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo(100, 6)
    root.renderer.nativeSimulateMouseUp(endX, endY)

    expect(commits).toHaveLength(1)
    // Available axis is 394px; the second pane is clamped to its 100px minimum.
    expect(commits[0]).toBeCloseTo(294 / 394, 6)
  })

  it.each([
    ["horizontal", { width: 400, height: 120 }, "width", 80, 100],
    ["vertical", { width: 120, height: 400 }, "height", 80, 100],
  ] as const)("clamps default, controlled, and changed bounds in the %s axis", (direction, style, axis, firstMinimum, secondMinimum) => {
    const root = createTestRoot()
    const commits: number[] = []
    const view = (ratio: number | undefined, size = style) => (
      <SplitView
        direction={direction}
        {...(ratio === undefined ? { defaultRatio: 0 } : { ratio })}
        minSize={firstMinimum}
        minSecondSize={secondMinimum}
        dividerSize={6}
        onResize={(value) => commits.push(value)}
        style={size}
      >
        <div testId="first" style={{ flexGrow: 1, backgroundColor: "#111111" }} />
        <div testId="second" style={{ flexGrow: 1, backgroundColor: "#222222" }} />
      </SplitView>
    )

    root.render(view(undefined))
    expect(rect(root, "first")[axis]).toBeCloseTo(firstMinimum, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo((direction === "horizontal" ? style.width : style.height) - 6 - firstMinimum, 6)
    root.renderer.nativeSimulateMouseDown(direction === "horizontal" ? firstMinimum + 3 : 60, direction === "horizontal" ? 60 : firstMinimum + 3)
    root.renderer.nativeSimulateMouseMove(direction === "horizontal" ? 20 : 60, direction === "horizontal" ? 60 : 20, 0)
    expect(rect(root, "first")[axis]).toBeCloseTo(firstMinimum, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo((direction === "horizontal" ? style.width : style.height) - 6 - firstMinimum, 6)
    root.renderer.nativeSimulateMouseMove(direction === "horizontal" ? 401 : 60, direction === "horizontal" ? 60 : 401, 0)

    root.render(view(1))
    root.renderer.nativeSimulateMouseDown(direction === "horizontal" ? style.width - 6 - secondMinimum + 3 : 60, direction === "horizontal" ? 60 : style.height - 6 - secondMinimum + 3)
    expect(rect(root, "first")[axis]).toBeCloseTo((direction === "horizontal" ? style.width : style.height) - 6 - secondMinimum, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo(secondMinimum, 6)

    const changed = direction === "horizontal" ? { width: 240, height: 120 } : { width: 120, height: 240 }
    root.render(view(1, changed))
    expect(rect(root, "first")[axis]).toBeCloseTo((direction === "horizontal" ? changed.width : changed.height) - 6 - secondMinimum, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo(secondMinimum, 6)
    expect(commits).toEqual([])
  })

  it.each([
    ["horizontal", { width: 160, height: 120 }, "width"],
    ["vertical", { width: 120, height: 160 }, "height"],
  ] as const)("preserves both minima and clips trailing overflow in an undersized %s axis", (direction, style, axis) => {
    const root = createTestRoot()
    const view = (ratio: number, size = style) => (
      <SplitView direction={direction} ratio={ratio} minSize={80} minSecondSize={100} dividerSize={6} style={size}>
        <div testId="first" style={{ flexGrow: 1 }} />
        <div testId="second" style={{ flexGrow: 1 }} />
      </SplitView>
    )

    root.render(view(0.5))
    expect(rect(root, "first")[axis]).toBeCloseTo(80, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo(100, 6)

    root.render(view(1))
    expect(rect(root, "first")[axis]).toBeCloseTo(154, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo(100, 6)

    const changed = direction === "horizontal" ? { width: 140, height: 120 } : { width: 120, height: 140 }
    root.render(view(0.5, changed))
    expect(rect(root, "first")[axis]).toBeCloseTo(80, 6)
    expect(rect(root, "second")[axis]).toBeCloseTo(100, 6)
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
