import { defineConfig } from "vitest/config"

// Every React test root owns a real Metal-backed native window. File-level
// serialization is the suite's single GPU-capacity policy.
export default defineConfig({
  test: {
    fileParallelism: false,
  },
})
