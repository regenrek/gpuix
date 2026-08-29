import { fileURLToPath } from "node:url"
import { describe, expect, it } from "vitest"
import { launch } from "../automation/index.js"

const describeMac = process.platform === "darwin" ? describe : describe.skip

describeMac("live automation stdio", () => {
  it("drives focused native input and cmd key events through one real host", async () => {
    const host = fileURLToPath(
      new URL("./fixtures/live-automation-host.tsx", import.meta.url)
    )
    const app = await launch({ command: "bun", args: [host] })

    try {
      const allText = await app.call("getAllText", {})
      const paintedText = await app.call("getPaintedText", {})

      expect(allText.text).toContain("Value: ")
      expect(paintedText.text).toContain("Type here")

      const { tree } = await app.call("getTree", {})
      const input = findTestId(tree, "live-automation-input")
      expect(input).toBeDefined()

      await app.call("keystrokes", { elementId: input!.id, keys: "h i" })
      expect((await app.call("getAllText", {})).text).toContain("Value: hi")

      await app.call("keystrokes", { elementId: input!.id, keys: "cmd-a" })
      await app.call("keyDown", { elementId: input!.id, key: "cmd-c" })
      expect((await app.call("getAllText", {})).text).toContain(
        "Command events: none"
      )
      await app.call("keyUp", { elementId: input!.id, key: "cmd-c" })
      expect((await app.call("getAllText", {})).text).toContain(
        "Command events: up:cmd-c"
      )
    } finally {
      await app.close()
    }
  })
})

function findTestId(
  node: { id: number; testId?: string; children?: unknown[] } | null,
  testId: string
): { id: number; testId?: string; children?: unknown[] } | undefined {
  if (!node) return undefined
  if (node.testId === testId) return node
  for (const child of node.children ?? []) {
    const found = findTestId(
      child as { id: number; testId?: string; children?: unknown[] },
      testId
    )
    if (found) return found
  }
  return undefined
}
