import React from "react"
import { describe, expect, it } from "vitest"
import type { EventPayload } from "@regenrek/gpuix-native"
import { createTestRoot, hasNativeTestRenderer } from "../testing.js"

const describeNative = hasNativeTestRenderer ? describe : describe.skip
const encode = (text: string) => Buffer.from(text).toString("base64")

describeNative("native terminal", () => {
  it("parses streamed VT output through the imperative renderer path", () => {
    const root = createTestRoot()
    root.render(<terminal style={{ width: 640, height: 240 }} />)
    const terminal = root.renderer.findByType("terminal")[0]

    root.renderer.terminalWrite(terminal.id, encode("plain "))
    root.renderer.terminalWrite(terminal.id, encode("\u001b[31mred\u001b[0m"))

    expect(root.renderer.getPaintedText().join("\n")).toContain("plain red")
  })

  it("resets emulator content before attaching a different terminal", () => {
    const root = createTestRoot()
    root.render(<terminal style={{ width: 640, height: 240 }} />)
    const terminal = root.renderer.findByType("terminal")[0]

    root.renderer.terminalWrite(terminal.id, encode("old terminal"))
    root.renderer.terminalReset(terminal.id)
    root.renderer.terminalWrite(terminal.id, encode("new terminal"))

    const painted = root.renderer.getPaintedText().join("\n")
    expect(painted).toContain("new terminal")
    expect(painted).not.toContain("old terminal")
  })

  it("emits base64 terminal bytes for native keyboard input", () => {
    const root = createTestRoot()
    const chunks: string[] = []
    root.render(
      <terminal
        style={{ width: 640, height: 240 }}
        onTerminalInput={(event: EventPayload) => {
          if (event.dataBase64) chunks.push(Buffer.from(event.dataBase64, "base64").toString())
        }}
      />,
    )
    const terminal = root.renderer.findByType("terminal")[0]

    root.renderer.nativeSimulateKeystrokes(terminal.id, "h i enter")

    expect(chunks.join("")).toBe("hi\r")
  })

  it("takes keyboard focus when its viewport is clicked", () => {
    const root = createTestRoot()
    const chunks: string[] = []
    root.render(
      <terminal
        style={{ width: 640, height: 240 }}
        onTerminalInput={(event: EventPayload) => {
          if (event.dataBase64) chunks.push(Buffer.from(event.dataBase64, "base64").toString())
        }}
      />,
    )
    const terminal = root.renderer.findByType("terminal")[0]
    root.renderer.flush()
    const bounds = root.renderer.getElementBounds(terminal.id)
    expect(bounds).not.toBeNull()
    if (!bounds) return

    root.renderer.nativeSimulateClick(bounds[0] + bounds[2] / 2, bounds[1] + bounds[3] / 2)
    root.renderer.simulateKeystrokes("h i enter")

    expect(chunks.join("")).toBe("hi\r")
  })

  it("commits composed Unicode through the platform IME handler", () => {
    const root = createTestRoot()
    const chunks: string[] = []
    root.render(
      <terminal
        style={{ width: 640, height: 240 }}
        onTerminalInput={(event: EventPayload) => {
          if (event.dataBase64) chunks.push(Buffer.from(event.dataBase64, "base64").toString())
        }}
      />,
    )
    const terminal = root.renderer.findByType("terminal")[0]

    root.renderer.nativeSimulateInput(terminal.id, "日本語")

    expect(chunks.join("")).toBe("日本語")
  })

  it("selects terminal output through GPUIX's canonical text selection", () => {
    const root = createTestRoot()
    root.render(<terminal style={{ width: 640, height: 240 }} />)
    const terminal = root.renderer.findByType("terminal")[0]
    root.renderer.terminalWrite(terminal.id, encode("select this"))
    root.renderer.flush()
    const bounds = root.renderer.getElementBounds(terminal.id)
    expect(bounds).not.toBeNull()
    if (!bounds) return

    const selected = root.renderer.dragSelect(
      bounds[0] + 8,
      bounds[1] + 10,
      bounds[0] + bounds[2] - 8,
      bounds[1] + 10,
    )

    expect(selected).toContain("select this")
  })

  it("reports the measured viewport instead of a host-side estimate", () => {
    const root = createTestRoot()
    root.render(
      <terminal
        style={{ width: 640, height: 240 }}
        onTerminalResize={() => undefined}
      />,
    )

    const resize = root.renderer
      .drainEvents()
      .find((event) => event.eventType === "terminalResize")
    expect(resize?.rows).toBeGreaterThan(2)
    expect(resize?.cols).toBeGreaterThan(2)
  })

  it("rejects terminal output aimed at a different retained element type", () => {
    const root = createTestRoot()
    root.render(<div />)
    const div = root.renderer.findByType("div")[0]

    expect(() => root.renderer.terminalWrite(div.id, encode("wrong target"))).toThrow(
      /not terminal/,
    )
  })
})
