import type { EventPayload } from "@regenrek/gpuix-native"
import type { Container, NativeRenderer } from "../types/host.js"
import { describe, expect, it, vi } from "vitest"

describe("hot reload runtime state", () => {
  it("routes an old native callback to handlers on the remounted root", async () => {
    vi.resetModules()
    const beforeReload = await import("../reconciler/event-registry.js")
    const dispatchFromNativeWindow = beforeReload.handleGpuixEvent

    vi.resetModules()
    const afterReload = await import("../reconciler/event-registry.js")
    const renderer = {} as NativeRenderer
    const onClick = vi.fn()
    const container = {
      renderer,
      ids: { nextElementId: 0 },
      eventHandlers: new Map([
        [17, new Map([["click", onClick]])],
      ]),
    } as Container

    try {
      afterReload.attachRoot(renderer, container)

      dispatchFromNativeWindow(
        { elementId: 17, eventType: "click" } as EventPayload,
        renderer
      )

      expect(onClick).toHaveBeenCalledOnce()
    } finally {
      afterReload.detachRoot(renderer)
    }
  })

  it("keeps element ids monotonic across module reloads", async () => {
    vi.resetModules()
    const beforeReload = await import("../reconciler/runtime-state.js")
    const renderer = {} as NativeRenderer
    const firstAllocator = beforeReload.idAllocatorForRenderer(renderer)
    firstAllocator.nextElementId = 41

    vi.resetModules()
    const afterReload = await import("../reconciler/runtime-state.js")

    expect(afterReload.idAllocatorForRenderer(renderer).nextElementId).toBe(41)
  })
})
