import fs from "fs"
import path from "path"
import React from "react"
import { beforeAll, describe, expect, it } from "vitest"

import { createTestRoot, hasNativeTestRenderer } from "../testing.js"
import { expectScreenshotsDiffer, expectScreenshotsEqual, SHOTS_DIR } from "./test-utils.js"

const describeNative = hasNativeTestRenderer ? describe : describe.skip
const shot = (name: string) => path.join(SHOTS_DIR, `canvas-${name}.png`)
beforeAll(() => fs.mkdirSync(SHOTS_DIR, { recursive: true }))

const particle = { type: "particle" as const, id: "dot", from: { x: 0.1, y: 0.5 }, to: { x: 0.9, y: 0.5 }, radius: 0.12, color: "#ff00ff", durationMs: 100 }
function Surface({ children }: { children: React.ReactNode }) { return <div style={{ width: 240, height: 160, backgroundColor: "#101010", padding: 20 }}>{children}</div> }

describeNative("canvas", () => {
  it("uses the native GPU path for all command types in source order, clips, and reports bounds", () => {
    const { render, renderer } = createTestRoot()
    const first = shot("ordered-first"), second = shot("ordered-second"), clippedBlank = shot("clipped-blank"), clipped = shot("clipped")
    const common = [
      { type: "line" as const, id: "line", from: { x: 0, y: 0 }, to: { x: 1, y: 1 }, width: 0.04, color: "#ff0000" },
      { type: "circle" as const, id: "circle", center: { x: 0.5, y: 0.5 }, radius: 0.2, color: "#0000ff" }, particle,
    ]
    const red = { type: "rect" as const, id: "red", x: 0.2, y: 0.2, width: 0.6, height: 0.6, radius: 0, color: "#ff0000" }
    const green = { type: "rect" as const, id: "green", x: 0.2, y: 0.2, width: 0.6, height: 0.6, radius: 0, color: "#00ff00" }
    render(<div style={{ width: 700, height: 440, backgroundColor: "#101010", padding: 20 }}><canvas testId="drawing" style={{ width: 640, height: 400 }} commands={[...common, red, green]} /></div>)
    const canvas = renderer.findByTestId("drawing")
    expect(renderer.getElementBounds(canvas!.id)).toEqual([20, 20, 640, 400])
    renderer.captureScreenshot(first)
    render(<div style={{ width: 700, height: 440, backgroundColor: "#101010", padding: 20 }}><canvas testId="drawing" style={{ width: 640, height: 400 }} commands={[...common, green, red]} /></div>)
    renderer.captureScreenshot(second); expectScreenshotsDiffer(first, second)
    render(<Surface><canvas testId="drawing" style={{ width: 0, height: 0, overflow: "visible" }} commands={[]} /></Surface>); renderer.captureScreenshot(clippedBlank)
    render(<Surface><canvas testId="drawing" style={{ width: 0, height: 0, overflow: "visible" }} commands={[{ ...particle, id: "clip", radius: 1, from: { x: 0, y: 0 }, to: { x: 0, y: 0 } }]} /></Surface>); renderer.captureScreenshot(clipped)
    expectScreenshotsEqual(clippedBlank, clipped)
  })

  it("atomically clears stale GPU pixels for capped, id, geometry, and particle rejections", () => {
    const { render, renderer } = createTestRoot(); const painted = shot("painted"), blank = shot("blank"), rejected = shot("rejected")
    const base = <Surface><canvas testId="drawing" style={{ width: 160, height: 100 }} commands={[particle]} /></Surface>
    render(base); renderer.captureScreenshot(painted)
    render(<Surface><canvas testId="drawing" style={{ width: 160, height: 100 }} commands={[]} /></Surface>); renderer.captureScreenshot(blank)
    const invalidSnapshots = [
      [{ ...particle, id: "duplicate" }, { ...particle, id: "duplicate" }],
      [{ ...particle, id: "geometry", radius: 2 }],
      Array.from({ length: 257 }, (_, index) => ({ ...particle, id: `particle-${index}` })),
      [{ ...particle, id: "x".repeat(256 * 1024) }],
    ]
    for (const commands of invalidSnapshots) {
      render(base); render(<Surface><canvas testId="drawing" style={{ width: 160, height: 100 }} commands={commands} /></Surface>); renderer.captureScreenshot(rejected)
      expectScreenshotsEqual(blank, rejected)
      expect(renderer.getElementBounds(renderer.findByTestId("drawing")!.id)).toEqual([20, 20, 160, 100])
    }
    expectScreenshotsDiffer(painted, blank)
  })

  it("uses the frozen native clock for particle progress, reset, pause, and visibility", () => {
    const { render, renderer } = createTestRoot(); const start = shot("particle-start"), advanced = shot("particle-advanced"), paused = shot("particle-paused"), hidden = shot("particle-hidden"), reset = shot("particle-reset")
    renderer.clockPause(); renderer.clockSet(0)
    render(<Surface><canvas testId="drawing" style={{ width: 160, height: 100 }} commands={[particle]} /></Surface>); renderer.captureScreenshot(start)
    renderer.clockFastForward(50); renderer.captureScreenshot(advanced); expectScreenshotsDiffer(start, advanced)
    render(<Surface><canvas testId="drawing" motion="paused" style={{ width: 160, height: 100 }} commands={[particle]} /></Surface>); renderer.captureScreenshot(paused)
    renderer.clockFastForward(50); renderer.captureScreenshot(hidden); expectScreenshotsEqual(paused, hidden)
    render(<Surface><canvas testId="drawing" visible={false} style={{ width: 160, height: 100 }} commands={[particle]} /></Surface>)
    expect(renderer.getElementBounds(renderer.findByTestId("drawing")!.id)).toBeNull()
    renderer.captureScreenshot(hidden); renderer.clockFastForward(50); renderer.captureScreenshot(paused); expectScreenshotsEqual(hidden, paused)
    render(<Surface><canvas testId="drawing" style={{ width: 160, height: 100 }} commands={[{ ...particle, id: "reset" }]} /></Surface>); renderer.captureScreenshot(reset); expectScreenshotsEqual(start, reset)
  })
})
