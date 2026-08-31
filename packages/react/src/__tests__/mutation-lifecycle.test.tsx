import { Suspense } from "react"
import { describe, expect, it, vi } from "vitest"
import { handleGpuixEvent } from "../reconciler/event-registry.js"
import { createRoot, flushSync } from "../reconciler/reconciler.js"
import { createTestRoot, hasNativeTestRenderer, TestRenderer } from "../testing.js"

const describeNative = hasNativeTestRenderer ? describe : describe.skip

describeNative("mutation lifecycle", () => {
  it("does not paint host nodes from an abandoned Suspense render", () => {
    const { render, renderer, unmount } = createTestRoot()
    const pending = new Promise<never>(() => {})

    function Suspend(): never {
      throw pending
    }

    try {
      render(
        <Suspense fallback={<text>fallback</text>}>
          <div>
            <text>abandoned</text>
          </div>
          <Suspend />
        </Suspense>
      )

      expect(renderer.getPaintedText()).toEqual(["fallback"])
    } finally {
      unmount()
    }
  })

  it("keeps unchanged event handlers registered across renders", () => {
    const { render, renderer, unmount } = createTestRoot()
    const onClick = vi.fn()
    const clickable = (
      <div style={{ width: 100, height: 100 }} onClick={onClick}>
        click
      </div>
    )

    try {
      render(clickable)
      renderer.nativeSimulateClick(10, 10)
      render(clickable)
      renderer.nativeSimulateClick(10, 10)

      expect(onClick).toHaveBeenCalledTimes(2)
    } finally {
      unmount()
    }
  })

  it("materializes raw div click listener membership changes after mount", () => {
    const { render, renderer, unmount } = createTestRoot()
    const onClick = vi.fn()
    const style = { width: 100, height: 100 }

    try {
      render(<div style={style}>click</div>)
      renderer.nativeSimulateClick(10, 10)
      expect(onClick).not.toHaveBeenCalled()

      render(
        <div style={style} onClick={onClick}>
          click
        </div>
      )
      renderer.nativeSimulateClick(10, 10)
      expect(onClick).toHaveBeenCalledTimes(1)

      render(<div style={style}>click</div>)
      renderer.nativeSimulateClick(10, 10)
      expect(onClick).toHaveBeenCalledTimes(1)
    } finally {
      unmount()
    }
  })

  it("unlinks then destroys a conditionally removed retained subtree in one mutation batch", () => {
    const renderer = new TestRenderer()
    const root = createRoot(renderer)
    const onRemovedClick = vi.fn()
    const tree = (showRemoved: boolean) => (
      <div testId="shell">
        <div testId="survivor">survivor</div>
        {showRemoved && (
          <div testId="removed" onClick={onRemovedClick}>
            <div testId="removed-descendant">removed descendant</div>
          </div>
        )}
      </div>
    )

    try {
      flushSync(() => root.render(tree(true)))
      renderer.flush()
      const shellId = renderer.findByTestId("shell")?.id
      const survivorId = renderer.findByTestId("survivor")?.id
      const removedId = renderer.findByTestId("removed")?.id
      const descendantId = renderer.findByTestId("removed-descendant")?.id
      expect(shellId).toBeDefined()
      expect(survivorId).toBeDefined()
      expect(removedId).toBeDefined()
      expect(descendantId).toBeDefined()

      const applyBatch = vi.spyOn(renderer, "applyBatch")
      flushSync(() => root.render(tree(false)))

      expect(applyBatch).toHaveBeenCalledTimes(1)
      const mutations = JSON.parse(applyBatch.mock.calls[0]![0]) as unknown[][]
      const unlinkIndex = mutations.findIndex(
        (mutation) => mutation[0] === "removeChild" && mutation[1] === shellId && mutation[2] === removedId
      )
      const destroyIndex = mutations.findIndex(
        (mutation) => mutation[0] === "destroyElement" && mutation[1] === removedId
      )
      expect(unlinkIndex).toBeGreaterThanOrEqual(0)
      expect(destroyIndex).toBe(unlinkIndex + 1)
      expect(renderer.getElement(shellId!)?.children).toEqual([survivorId])
      expect(renderer.getElement(removedId!)).toBeUndefined()
      expect(renderer.getElement(descendantId!)).toBeUndefined()

      handleGpuixEvent({ elementId: removedId!, eventType: "click" }, renderer)
      expect(onRemovedClick).not.toHaveBeenCalled()
    } finally {
      root.unmount()
    }
  })

  it("keeps element ids and click handlers isolated across live roots", () => {
    const a = createTestRoot()
    const b = createTestRoot()
    const onA = vi.fn()
    const onB = vi.fn()

    try {
      a.render(
        <div style={{ width: 100, height: 100 }} onClick={onA}>
          a
        </div>
      )
      b.render(
        <div style={{ width: 100, height: 100 }} onClick={onB}>
          b
        </div>
      )

      expect(a.renderer.findByType("text")[0]?.id).toBe(1)
      expect(b.renderer.findByType("text")[0]?.id).toBe(1)
      expect(a.renderer.getRoot()?.id).toBe(2)
      expect(b.renderer.getRoot()?.id).toBe(2)

      const click = { elementId: 2, eventType: "click" }
      handleGpuixEvent(click, a.renderer)
      expect(onA).toHaveBeenCalledTimes(1)
      expect(onB).not.toHaveBeenCalled()

      handleGpuixEvent(click, b.renderer)
      expect(onA).toHaveBeenCalledTimes(1)
      expect(onB).toHaveBeenCalledTimes(1)
    } finally {
      a.unmount()
      b.unmount()
    }
  })

  it("does not give a remounted root the old element ids", () => {
    const renderer = new TestRenderer()
    const first = createRoot(renderer)
    const onFirst = vi.fn()
    const onSecond = vi.fn()
    const tree = (onClick: () => void) => (
      <div style={{ width: 100, height: 100 }} onClick={onClick}>
        row
      </div>
    )

    flushSync(() => first.render(tree(onFirst)))
    renderer.flush()
    const firstRootId = renderer.getRoot()?.id
    expect(firstRootId).toBe(2)
    first.unmount()

    const second = createRoot(renderer)
    try {
      flushSync(() => second.render(tree(onSecond)))
      renderer.flush()
      expect(renderer.getRoot()?.id).not.toBe(firstRootId)

      handleGpuixEvent({ elementId: firstRootId!, eventType: "click" }, renderer)
      expect(onSecond).not.toHaveBeenCalled()
    } finally {
      second.unmount()
    }
  })
})
