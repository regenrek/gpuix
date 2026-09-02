import { describe, expect, it } from "vitest"
import type { GpuixRenderer } from "@regenrek/gpuix-native"

import { TestRenderer } from "../testing.js"

type NativeAppshotContract = Pick<
  GpuixRenderer,
  | "preflightAppshotPermission"
  | "requestAppshotPermission"
  | "selectAppshotWindow"
  | "captureAppshotWindow"
  | "captureFrontmostAppshot"
  | "disposeAppshotWindow"
  | "createAppshotPreview"
  | "disposeAppshotPreview"
>

describe("appshot", () => {
  it("keeps selected sources behind one-shot opaque handles", async () => {
    const renderer = new TestRenderer()
    renderer.setAppshotSelection(true)

    const selection = await renderer.selectAppshotWindow()
    expect(selection.status).toBe("selected")
    expect(selection.handle).toMatch(/^appshot-\d+$/)
    expect(selection).not.toHaveProperty("title")
    expect(selection).not.toHaveProperty("bundleId")

    const png = await renderer.captureAppshotWindow(selection.handle!)
    expect([...png.subarray(0, 8)]).toEqual([137, 80, 78, 71, 13, 10, 26, 10])
    expect(png.subarray(12, 16).toString()).toBe("IHDR")
    await expect(renderer.captureAppshotWindow(selection.handle!)).rejects.toThrow("unavailable")
  })

  it("models cancellation and permission without platform metadata", async () => {
    const renderer = new TestRenderer()
    renderer.setAppshotPermission(false)
    expect(renderer.preflightAppshotPermission()).toEqual({
      status: "missing",
      restartRequired: false,
    })
    await expect(renderer.selectAppshotWindow()).resolves.toEqual({ status: "cancelled" })
  })

  it("captures into a native preview, renders its opaque handle, then disposes it", async () => {
    const renderer = new TestRenderer()
    renderer.setAppshotSelection(true)
    const selected = await renderer.selectAppshotWindow()
    const png = await renderer.captureAppshotWindow(selected.handle!)
    const preview = renderer.createAppshotPreview(png)

    expect(preview).toMatch(/^appshot-preview-\d+$/)
    expect(preview).not.toContain("/")
    expect(preview).not.toContain("data:")
    expect(preview).not.toContain("base64")

    renderer.createElement(1, "img")
    renderer.setCustomProp(1, "appshotPreviewHandle", JSON.stringify(preview))
    renderer.setStyle(1, JSON.stringify({ width: 160, height: 90 }))
    renderer.setRoot(1)
    renderer.commitMutations()
    renderer.flush()

    const image = renderer.findByType("img")[0]
    expect(image.customProps).toEqual({ appshotPreviewHandle: preview })
    expect(image.customProps).not.toHaveProperty("src")
    renderer.disposeAppshotPreview(preview)
    renderer.flush()
  })
})
