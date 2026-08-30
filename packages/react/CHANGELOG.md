# @regenrek/gpuix-react

## 0.5.0-regenrek.5

### Patch Changes

- Export the browser-surface event types and JSX intrinsic contract from the React package.
- Updated dependencies
  - @regenrek/gpuix-native@0.5.0-regenrek.5

## 0.5.0-regenrek.4

### Minor Changes

- Add explicit host decisions for browser navigation and downloads.

### Patch Changes

- Updated dependencies
  - @regenrek/gpuix-native@0.5.0-regenrek.4

## 0.5.0-regenrek.3

### Minor Changes

- Add the macOS `browser-surface` custom element with native WKWebView composition, isolated persistent profiles, typed navigation and loading events, download observation, browser-data clearing, focus, and navigation commands.

  GPUI now draws base and overlay scenes around native browser views. Interactive overlay regions receive input while transparent overlay areas pass input through to the browser.

### Patch Changes

- Updated dependencies
  - @regenrek/gpuix-native@0.5.0-regenrek.3

## 0.5.0-regenrek.2

### Patch Changes

- Expose the desktop-native clipboard write contract through React's renderer and deterministic test adapter.
- @regenrek/gpuix-native@0.5.0-regenrek.2

## 0.5.0-regenrek.1

### Patch Changes

- Updated dependencies
  - @regenrek/gpuix-native@0.5.0-regenrek.1

## 0.5.0-regenrek.0

### Minor Changes

- dcddaf3: Add per-side border widths and one structured `boxShadow` style with offset, blur, spread, and color values.
- 5a92f02: GitHub releases now include a standalone **chat example** executable for each platform.
  
  Download `example-chat-*` from the [GitHub release](https://github.com/regenrek/gpuix/releases). No Node, Bun, or Rust install is required.
  
  ```bash
  chmod +x example-chat-aarch64-apple-darwin
  ./example-chat-aarch64-apple-darwin
  ```
  
  macOS may block the unsigned binary the first time. Right-click the file, choose Open, and confirm.
  
  Windows: download `example-chat-x86_64-pc-windows-msvc.exe` and double-click it.
- 96f9569: Add `getDebugFrameOverlayStats()` so tests and apps can read the same draw times the on-screen overlay shows.
  
  ```ts
  renderer.resetDebugFrameOverlayStats()
  // ... scroll or click ...
  const stats = renderer.getDebugFrameOverlayStats()
  // stats.currentMs, stats.p90Ms, stats.p99Ms, stats.maxMs, stats.frames, stats.samples
  ```
  
  `p90Ms` is the overlay **10%** line. `p99Ms` is the **1%** line. Those are the slow tail, not the fast frames.
  
  The chat example uses this in `examples/chat.perf.test.tsx` to catch mount, wheel, and sidebar regressions.
- 9df699a: Accept the full csscolorparser 0.8.3 string grammar across styles, themes,
  pseudo-states, selection colors, SVG tint, borders, and shadows. This adds
  modern RGB/HSL/HWB, HSV, LAB/LCH, OKLab/OKLCH, named, transparent, alpha,
  `none`, and limited relative-color forms without changing TypeScript types.
- Add a bounded native retained canvas intrinsic with deterministic automation time.
- Add privacy-preserving native Appshot permission, opaque source-handle, capture, disposal, and shortcut contracts.
- b07225e: Add `SplitView`, a generic two-pane native GPUI layout. It reserves its divider
  in pane geometry, clamps both pane minimums, and keeps capture, hit testing,
  cursor feedback, painting, continuous geometry, cancellation, and cleanup in
  Rust. React receives one final ratio only when a drag commits.
  
  ```tsx
  import { SplitView } from '@regenrek/gpuix-react'
  
  <SplitView minSize={240} minSecondSize={320} onResize={setSidebarRatio}>
    <Sidebar />
    <Main />
  </SplitView>
  ```
- bcfcaa9: Render React GPUIX apps through GPUI's browser platform with WebGPU and a WebGL2 fallback.
  
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
- Add `promptForDirectory()` to open the platform-native single-directory picker. The promise resolves to the selected path or `null` when the dialog is cancelled.
- 96f9569: Add `THROTTLE` for macOS CPU clamps on profile runs.
  
  `THROTTLE=utility` restarts the process under `taskpolicy -c utility`. That pins work to E-cores. Use it as an M1/M2 Air CPU proxy. `background` and `maintenance` are slower.
  
  ```bash
  THROTTLE=utility bun run test chat.perf.test.tsx
  THROTTLE=utility bun --hot chat.tsx
  ```
  
  This is not Chrome 6x. GPU and RAM stay on the host machine. Do not set `THROTTLE` in CI.
- 2bf8088: Add a windowed `VirtualList` so long transcripts do not create every React row at mount.
  
  Pass `itemCount` and `renderItem`. Native keeps the full logical length for the scrollbar. React only mounts the visible window plus overdraw.
  
  ```tsx
  import { VirtualList } from '@regenrek/gpuix-react'
  
  <VirtualList
    itemCount={turns.length}
    estimatedItemHeight={220}
    renderItem={(index) => <ChatTurn key={turns[index].id} turn={turns[index]} />}
  />
  ```
  
  The host `<virtual-list>` still accepts a full `children` map. Use `VirtualList` when the first mount of thousands of rows is too slow.
  
  `onVisibleRange` reports `startIndex` and `endIndex` after a scroll.

### Patch Changes

- 9c9deba: Publish `@regenrek/gpuix-native` and `@regenrek/gpuix-react` as **Apache-2.0**.
  
  The packages now declare `license: Apache-2.0` and ship the license text in the npm tarball. GPUI itself is Apache-2.0, so this matches the native dependency.
- f859d1b: Keep abandoned concurrent renders out of the native mutation queue.
  
  React may throw away a Suspense render. GPUIX now waits until commit before it creates native elements, so fallback text paints and abandoned text does not.
  
  Unchanged click handlers also stay registered across rerenders. GPUIX no longer clears the whole handler map before every update.
- Keep the windowed `VirtualList` mounted range at the tail when rows are appended while tail following is active.
- 8d49402: Reject automation calls after close and make session shutdown idempotent across in-process and SSE backends.
- 75e304e: Hide GPUI Web's IME text bridge so browser hosts no longer show a native input at the top of the page.
  
  GPUI Web keeps a focused DOM `<input data-gpui-input>` for clipboard, keyboard, and composition. It sat at `top: 0` with only a 1px opacity-0 box, so host `input` CSS could unhide it. The control now uses important inline hide styles, `clip-path`, and `autocomplete="off"`. It stays focusable for IME.
  
  ```html
  <!-- still present for IME, no longer painted -->
  <input data-gpui-input autocomplete="off" />
  ```
- a0a84bf: Give each React root its own event handler map. Two `createTestRoot()` trees can both start at id `1` without overwriting each other's handlers. `resetIdCounter()` is gone.
  
  A remount on the same native renderer keeps allocating new ids. A late event from the old tree cannot hit a new handler that reused id `1`.
  
  `handleGpuixEvent` now needs the renderer that produced the event:
  
  ```ts
  handleGpuixEvent(event, renderer)
  ```
- Keep native event routing and element ID allocation connected to the active React root after a Bun hot reload.
- 727946d: Make native `<markdown>` wrap in flex columns, and record painted bounds on `<markdown>`, `<code>`, and `<diff>`.
  
  A markdown node in a flex row used to keep its max-content width, so a long paragraph or list item could blow past the parent. The root and each text block now shrink with `min-width: 0`, the same rule list items already used.
  
  ```tsx
  <div style={{ display: 'flex', flexDirection: 'row', width: 280 }}>
    <div style={{ width: 40, flexShrink: 0 }} />
    <markdown
      source="- a long sentence that must wrap in the remaining column"
      style={{ flexGrow: 1 }}
    />
  </div>
  ```
  
  Fenced code inside `<markdown>` now matches `<code>`: long lines scroll on X and leave the vertical wheel on the parent. Before this they clipped at the rounded card.
  
  `getElementBounds` and automation locators also work on those three elements, including an empty `<markdown source="" />`. They never painted a bounds tracker, so a `testId` on `<markdown>` returned null.
  
  `TestRenderer.findByTestId()` looks up that `testId` from the retained tree.
- Updated dependencies [9c9deba]
- Updated dependencies [dcddaf3]
- Updated dependencies [a476ce4]
- Updated dependencies [e9a2416]
- Updated dependencies [ce298d4]
- Updated dependencies [16f2de1]
- Updated dependencies [6d2fc40]
- Updated dependencies [96f9569]
- Updated dependencies [9df699a]
- Updated dependencies
- Updated dependencies [75e304e]
- Updated dependencies
- Updated dependencies
- Updated dependencies [b07225e]
- Updated dependencies
- Updated dependencies [bcfcaa9]
- Updated dependencies [41dea57]
- Updated dependencies [001d7d4]
- Updated dependencies
- Updated dependencies [3cc4196]
- Updated dependencies [2bf8088]
- Updated dependencies [727946d]
  - @regenrek/gpuix-native@0.5.0-regenrek.0
