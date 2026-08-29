---
'@regenrek/gpuix-native': minor
'@regenrek/gpuix-react': minor
---

Render React GPUIX apps through GPUI's browser platform with WebGPU and a WebGL2 fallback.

```sh
bun run web
```

The browser build exposes the same mutation and event interfaces to React and reuses `RetainedTree`, `GpuixView`, styles, and text painting. The web example includes the complete chat transcript with React-composed Markdown, a table, code, and a native diff. Sidebar controls, selects, comboboxes, inputs, and other event-driven components work through an asynchronous Wasm-to-JavaScript callback bridge. Browser copy consumes the native shortcut after writing selected GPUIX text, while paste stays on the DOM clipboard event so text reaches the focused input. napi-rs remains the desktop bridge; wasm-bindgen starts `gpui_web` in the browser. Raw SVG sources render through GPUI's monochrome icon pipeline, and automation, motion, and GPUI scroll gestures use browser-safe clocks. Syntect's pure-Rust fancy-regex engine highlights code, diffs, and Markdown fences on Wasm with the same theme palette as desktop.

Browser apps always expose the Playwright-like automation API through `globalThis.gpuix`:

```ts
await globalThis.gpuix.getByTestId('send').click()
await globalThis.gpuix.getByTestId('composer').fill('hello')
await globalThis.gpuix.clock.fastForward(200)
```

The global uses the same `App`, `Locator`, schemas, and in-process backend as native tests. Browser tools such as Playwriter can evaluate these calls directly in the page.
