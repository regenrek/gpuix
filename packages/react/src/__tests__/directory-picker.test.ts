import { describe, expect, it } from "vitest"

import { TestRenderer } from "../testing.js"

describe("directory picker", () => {
  it("returns one configured path and resets to cancellation", async () => {
    const renderer = new TestRenderer()
    renderer.setDirectoryPromptResponse("/tmp/example")

    await expect(renderer.promptForDirectory()).resolves.toBe("/tmp/example")
    await expect(renderer.promptForDirectory()).resolves.toBeNull()
  })
})
