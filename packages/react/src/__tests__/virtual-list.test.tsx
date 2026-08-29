/// The native <virtual-list>: lazy rows, programmatic scrolling, and chat tail following.

import React from "react"
import { describe, expect, it } from "vitest"
import { VirtualList } from "../components/index.js"
import { createTestRoot } from "../testing.js"

function Rows({ count }: { count: number }) {
  return Array.from({ length: count }, (_, index) => (
    <div
      key={index}
      style={{
        display: "flex",
        height: 40,
        flexShrink: 0,
        alignItems: "center",
      }}
    >
      <text>{`row-${index}`}</text>
    </div>
  ))
}

function FocusableRows({ inputIndex = 0 }: { inputIndex?: number }) {
  const [value, setValue] = React.useState("")
  return (
    <virtual-list
      overdraw={0}
      estimatedItemHeight={40}
      style={{ width: 400, height: 160 }}
    >
      {Array.from({ length: 30 }, (_, index) => (
        <div key={index} style={{ height: 40, flexShrink: 0 }}>
          {index === inputIndex ? (
            <input
              autoFocus
              placeholder="focused-input"
              value={value}
              onChange={(event) => setValue(event.value ?? "")}
            />
          ) : (
            <text>{`row-${index}`}</text>
          )}
        </div>
      ))}
    </virtual-list>
  )
}

function DynamicFocusableRows({ enabled }: { enabled: boolean }) {
  const [value, setValue] = React.useState("")
  return (
    <virtual-list
      overdraw={0}
      estimatedItemHeight={40}
      style={{ width: 400, height: 160 }}
    >
      {Array.from({ length: 30 }, (_, index) => (
        <div key={index} style={{ height: 40, flexShrink: 0 }}>
          {index === 0 && enabled ? (
            <input
              autoFocus
              value={value}
              onChange={(event) => setValue(event.value ?? "")}
            />
          ) : (
            <text>{`row-${index}`}</text>
          )}
        </div>
      ))}
    </virtual-list>
  )
}

describe("<virtual-list>", () => {
  it("builds and paints only rows near the viewport", () => {
    const { render, renderer } = createTestRoot()
    render(
      <virtual-list
        overdraw={0}
        estimatedItemHeight={40}
        style={{ width: 400, height: 160 }}
      >
        <Rows count={100} />
      </virtual-list>
    )

    expect(renderer.getAllText()).toHaveLength(100)

    const painted = renderer.getPaintedText()
    expect(painted).toContain("row-0")
    expect(painted).not.toContain("row-99")
    expect(painted.length).toBeLessThan(10)
  })

  it("builds a distant row when it is scrolled into view", () => {
    const { render, renderer } = createTestRoot()
    render(
      <virtual-list
        overdraw={0}
        estimatedItemHeight={40}
        style={{ width: 400, height: 160 }}
      >
        <Rows count={100} />
      </virtual-list>
    )

    const list = renderer.findByType("virtual-list")[0]
    expect(list.children).toHaveLength(100)
    renderer.scrollToItem(list.id, 99)
    expect(renderer.getScrollOffset(list.id)?.[1]).toBeLessThan(-100)

    const painted = renderer.getPaintedText()
    expect(painted).toContain("row-99")
    expect(painted).not.toContain("row-0")
  })

  it("lazily builds custom elements inside rows", () => {
    const { render, renderer } = createTestRoot()
    render(
      <virtual-list
        overdraw={0}
        estimatedItemHeight={80}
        style={{ width: 400, height: 160 }}
      >
        {Array.from({ length: 30 }, (_, index) => (
          <div key={index} style={{ minHeight: 80, flexShrink: 0 }}>
            {index === 20 ? <markdown source="# Lazy markdown" /> : <text>{`row-${index}`}</text>}
          </div>
        ))}
      </virtual-list>
    )

    expect(renderer.findByType("markdown")).toHaveLength(1)
    expect(renderer.getPaintedText()).not.toContain("Lazy markdown")

    const list = renderer.findByType("virtual-list")[0]
    renderer.scrollToItem(list.id, 20)
    expect(renderer.getPaintedText()).toContain("Lazy markdown")
  })

  it("keeps a focused row active when it scrolls offscreen", () => {
    const { render, renderer } = createTestRoot()
    render(<FocusableRows />)

    const input = renderer.findByType("input")[0]
    renderer.simulateKeystrokes("a")
    expect(renderer.getElement(input.id)?.customProps?.value).toBe("a")

    const list = renderer.findByType("virtual-list")[0]
    renderer.scrollToItem(list.id, 29)
    renderer.simulateKeystrokes("b")
    expect(renderer.getElement(input.id)?.customProps?.value).toBe("ab")
  })

  it("reveals an initially focused offscreen row", () => {
    const { render, renderer } = createTestRoot()
    render(<FocusableRows inputIndex={20} />)

    expect(renderer.getPaintedText()).toContain("focused-input")

    const input = renderer.findByType("input")[0]
    renderer.simulateKeystrokes("a")
    expect(renderer.getElement(input.id)?.customProps?.value).toBe("a")
  })

  it("updates focus retention when an existing row becomes focusable", () => {
    const { render, renderer } = createTestRoot()
    render(<DynamicFocusableRows enabled={false} />)
    render(<DynamicFocusableRows enabled />)

    const input = renderer.findByType("input")[0]
    const list = renderer.findByType("virtual-list")[0]
    renderer.scrollToItem(list.id, 29)
    renderer.simulateKeystrokes("a")

    expect(renderer.getElement(input.id)?.customProps?.value).toBe("a")
  })

  it("follows appended chat rows while tail following is active", () => {
    const { render, renderer } = createTestRoot()
    const transcript = (count: number) => (
      <virtual-list
        alignment="bottom"
        followTail
        overdraw={0}
        estimatedItemHeight={40}
        style={{ width: 400, height: 160 }}
      >
        <Rows count={count} />
      </virtual-list>
    )

    render(transcript(20))
    expect(renderer.getPaintedText()).toContain("row-19")
    expect(renderer.getPaintedText()).not.toContain("row-0")

    render(transcript(21))
    expect(renderer.getPaintedText()).toContain("row-20")
    expect(renderer.getPaintedText()).not.toContain("row-0")
  })

  it("mounts appended rows in the windowed wrapper while tail following is active", () => {
    const { render, renderer } = createTestRoot()
    const transcript = (count: number) => (
      <VirtualList
        itemCount={count}
        renderItem={(index) => <div key={index}><text>{`row-${index}`}</text></div>}
        alignment="bottom"
        followTail
        overdraw={0}
        estimatedItemHeight={40}
        style={{ width: 400, height: 160 }}
      />
    )

    render(transcript(1))
    expect(renderer.getAllText()).toContain("row-0")

    render(transcript(2))
    expect(renderer.getAllText()).toContain("row-1")
  })

  it("keeps only the mounted window in the retained tree", () => {
    const { render, renderer } = createTestRoot()
    const windowed = (start: number) => (
      <virtual-list
        itemCount={1000}
        windowStart={start}
        overdraw={0}
        estimatedItemHeight={40}
        style={{ width: 400, height: 160 }}
      >
        {Array.from({ length: 8 }, (_, offset) => (
          <div
            key={start + offset}
            style={{
              display: "flex",
              height: 40,
              flexShrink: 0,
              alignItems: "center",
            }}
          >
            <text>{`row-${start + offset}`}</text>
          </div>
        ))}
      </virtual-list>
    )

    render(windowed(0))
    const list = renderer.findByType("virtual-list")[0]
    expect(list.children).toHaveLength(8)
    expect(renderer.getAllText()).toEqual([
      "row-0",
      "row-1",
      "row-2",
      "row-3",
      "row-4",
      "row-5",
      "row-6",
      "row-7",
    ])
    expect(renderer.getPaintedText()).toContain("row-0")

    render(windowed(50))
    expect(renderer.findByType("virtual-list")[0].children).toHaveLength(8)
    expect(renderer.getAllText()).toEqual([
      "row-50",
      "row-51",
      "row-52",
      "row-53",
      "row-54",
      "row-55",
      "row-56",
      "row-57",
    ])

    renderer.scrollToItem(list.id, 50)
    expect(renderer.getPaintedText()).toContain("row-50")
    expect(renderer.getPaintedText()).not.toContain("row-0")
  })

  it("lets overflow-x inside a row pan without moving the list", () => {
    const { render, renderer } = createTestRoot()
    render(
      <virtual-list
        overdraw={0}
        estimatedItemHeight={80}
        style={{ width: 240, height: 160 }}
      >
        <div style={{ width: "100%", height: 80, overflowX: "scroll" }}>
          <div style={{ width: 800, height: 80, flexShrink: 0 }}>
            <text>wide row</text>
          </div>
        </div>
        <div style={{ height: 80 }}>
          <text>below</text>
        </div>
      </virtual-list>
    )

    const list = renderer.findByType("virtual-list")[0]
    const scroller = renderer
      .findByType("div")
      .find((d) => d.style.overflowX === "scroll")!
    expect(renderer.getScrollOffset(scroller.id)?.[0] ?? 0).toBe(0)

    renderer.nativeSimulateScrollWheel(80, 40, -80, 0)
    const listOffset = renderer.getScrollOffset(list.id)
    const rowOffset = renderer.getScrollOffset(scroller.id)
    expect(listOffset?.[1] ?? 0, `list ${JSON.stringify(listOffset)}`).toBeCloseTo(0)
    expect(rowOffset?.[0], `row ${JSON.stringify(rowOffset)}`).toBeLessThan(0)
  })

})
