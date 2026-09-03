# @regenrek/gpuix-native

## 0.5.0-regenrek.15

## 0.5.0-regenrek.14

## 0.5.0-regenrek.13

### Minor Changes

- Add compact, theme-aware DockWorkspace tabs that share available width, truncate long labels, and keep secondary controls on the active tab.

## 0.5.0-regenrek.12

### Patch Changes

- Run application cleanup before the last embedded macOS window exits.

## 0.5.0-regenrek.11

## 0.5.0-regenrek.10

### Patch Changes

- Restore GPUI keyboard focus when a user clicks native GPUI content after interacting with an embedded browser.

## 0.5.0-regenrek.9

### Patch Changes

- Expose live scroll-wheel automation through GPUI's production input path.

## 0.5.0-regenrek.8

- Add opaque renderer-lifetime Appshot preview handles on macOS.
- Add native two-phase browser download destination selection.

## 0.5.0-regenrek.6

### Patch Changes

- Report WebKit's download intent and MIME-display capability in browser action requests so the embedding host can choose whether a navigation becomes a download.

## 0.5.0-regenrek.5

### Patch Changes

- Export the browser-surface event types and JSX intrinsic contract from the React package.

## 0.5.0-regenrek.4

### Minor Changes

- Add explicit host decisions for browser navigation and downloads.

## 0.5.0-regenrek.3

### Minor Changes

- Add the macOS `browser-surface` custom element with native WKWebView composition, isolated persistent profiles, typed navigation and loading events, download observation, browser-data clearing, focus, and navigation commands.

  GPUI now draws base and overlay scenes around native browser views. Interactive overlay regions receive input while transparent overlay areas pass input through to the browser.

## 0.5.0-regenrek.2

## 0.5.0-regenrek.1

### Patch Changes

- Add `GpuixRenderer.writeClipboardText(text)` for desktop-native clipboard writes.

## 0.5.0-regenrek.0

### Minor Changes

- dcddaf3: Add per-side border widths and one structured `boxShadow` style with offset, blur, spread, and color values.
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
- 3cc4196: Replace Tree-sitter with **Syntect** for native syntax highlighting. `<code>`, `<diff>`, and Markdown fences still detect language the same way and still paint from the theme palette. Token classes stay `HighlightKind` values, not baked-in colours, so a theme change recolours existing spans without a reparse.
  
  The highlighter now uses Syntect's **pure-Rust fancy-regex** engine. There is no Tree-sitter runtime and no per-language C grammar in the native binary.
  
  ```tsx
  <code code={source} language="typescript" />
  <markdown text={'```rust\nfn main() {}\n```'} />
  <diff patch={unified} />
  ```
  
  Language detection is unchanged: fence tag, then path, then shebang. Unknown languages still render as plain text.
  
  Token colours can shift a little versus Tree-sitter, because Syntect scopes are not the old capture names. The public `HighlightKind` contract and the JS theme override path are the same.
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
- a476ce4: Apply per-corner radii, `flexBasis`, and `alignContent` styles that were already declared in the public React style type.
- e9a2416: Synchronize custom-element props only when retained values change, avoiding repeated parsing and allocation on every frame.
- ce298d4: Enable live-app mouse input, locator bounds, and timeline clock controls on Windows, Linux, and FreeBSD.
- 16f2de1: Fix word navigation in macOS browsers so Option+Left and Option+Right move the input caret between words. GPUIX now selects macOS bindings from the browser platform at runtime, while other browsers and desktop platforms keep their existing shortcuts.
- 6d2fc40: Fix Windows x64 native binding failing to load with `ERR_DLOPEN_FAILED`.
  
  `require('@regenrek/gpuix-native')` no longer dies with `LoadLibrary failed: The specified procedure could not be found`. The published `.node` was statically importing `TaskDialogIndirect` from comctl32 v6 and `u_strlen` from `icuuc.dll`. Node and Bun do not activate comctl32 v6, so Windows resolved the old comctl32 and `LoadLibrary` failed before any JS ran.
  
  ```bash
  bun -e "require('@regenrek/gpuix-native'); console.log('OK')"
  ```
  
  Fixes #1
  Closes #2
- 75e304e: Hide GPUI Web's IME text bridge so browser hosts no longer show a native input at the top of the page.
  
  GPUI Web keeps a focused DOM `<input data-gpui-input>` for clipboard, keyboard, and composition. It sat at `top: 0` with only a 1px opacity-0 box, so host `input` CSS could unhide it. The control now uses important inline hide styles, `clip-path`, and `autocomplete="off"`. It stays focusable for IME.
  
  ```html
  <!-- still present for IME, no longer painted -->
  <input data-gpui-input autocomplete="off" />
  ```
- Keep inputs and other interactive controls inside `DockWorkspace` pointer-accessible while retaining native tab dragging and split resizing.
- Render bounded base64 image data URIs directly from memory in the native `img` host element.
- 41dea57: Speed up long virtual lists and automation locators.
  
  A 5,000-row chat used to rebuild virtual-list focus maps on every GPUI frame, even when the row ids had not changed. Sidebar motion and caret blink then paid that cost on every tick. Unchanged lists now return before that work.
  
  `getAutomationTree()` also stops serializing style, events, and custom props. Locators only need `id`, `type`, `testId`, `text`, and bounds. On a 5k-row tree that dropped tree JSON from about 110ms to about 22ms, so `getByTestId().click()` is no longer dominated by encoding unused style maps.
- 001d7d4: Start a text selection from the empty space before the glyphs.
  
  A press in parent padding, a code gutter, or the empty start of a line now clamps to the nearest text on that row. Before this, the mouse-down had to land inside the tight `TextLayout` box, so a drag that started just before the first character selected nothing.
  
  ```
    [padding] hello world
        ^
        press here, drag right  →  "hello world"
  ```
  
  A press above or below every line still does not start a selection. That keeps a composer or titlebar from claiming the nearest paragraph. A click without movement still selects nothing.
  
  `userSelect: "none"` now also blocks the start. A sidebar or other chrome on the same row as a paragraph will not start a selection on that paragraph. Native `<input>` and `<textarea>` own their own selection and do the same.
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
