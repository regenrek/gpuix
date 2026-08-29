import { describe, expect, it } from "vitest"
import type { GpuixRenderer } from "@regenrek/gpuix-native"

import { TestRenderer } from "../testing.js"

type NativeClipboardContract = Pick<GpuixRenderer, "writeClipboardText">

describe("clipboard writes", () => {
  it("records exact writes in order and clears them when taken", () => {
    const renderer = new TestRenderer()
    const clipboard: NativeClipboardContract = renderer

    clipboard.writeClipboardText("first payload")
    clipboard.writeClipboardText("second payload")

    expect(renderer.takeClipboardWrites()).toEqual([
      "first payload",
      "second payload",
    ])
    expect(renderer.takeClipboardWrites()).toEqual([])
  })
})
