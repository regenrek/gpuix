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
  | "registerGlobalShortcut"
  | "unregisterGlobalShortcut"
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

  it("dispatches and removes renderer-lifetime shortcuts through the native event owner", () => {
    const renderer = new TestRenderer()
    const token = renderer.registerGlobalShortcut("cmd-shift-9")
    renderer.triggerGlobalShortcut(token)
    expect(renderer.drainEvents()).toEqual([
      expect.objectContaining({ eventType: "globalShortcut", value: token }),
    ])
    expect(() => renderer.registerGlobalShortcut("cmd-shift-9")).toThrow()
    renderer.unregisterGlobalShortcut(token)
    expect(() => renderer.triggerGlobalShortcut(token)).toThrow("unavailable")
  })

  it("keeps shortcut callbacks renderer-local while consuming the native appshot contract", () => {
    const first = new TestRenderer()
    const second = new TestRenderer()
    const firstContract: NativeAppshotContract = first
    const secondContract: NativeAppshotContract = second
    const firstToken = firstContract.registerGlobalShortcut("cmd-shift-8")
    const secondToken = secondContract.registerGlobalShortcut("cmd-shift-8")

    first.triggerGlobalShortcut(firstToken)
    expect(first.drainEvents()).toEqual([
      expect.objectContaining({ eventType: "globalShortcut", value: firstToken }),
    ])
    expect(second.drainEvents()).toEqual([])

    second.triggerGlobalShortcut(secondToken)
    expect(second.drainEvents()).toEqual([
      expect.objectContaining({ eventType: "globalShortcut", value: secondToken }),
    ])
  })
})
