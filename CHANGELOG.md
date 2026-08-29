# Changelog

## 0.4.0

1. **Native `motion.div` animations** — animate from an initial style to a target style. React sends the targets once. Rust interpolates the presentation style and requests GPUI frames. The React tree is not reconciled on each frame.

   ```tsx
   import { motion } from '@regenrek/gpuix-react'

   <motion.div
     initial={{ width: 0, opacity: 0 }}
     animate={{ width: 260, opacity: 1 }}
     transition={{ duration: 0.2, ease: 'easeOut' }}
   >
     Sidebar content
   </motion.div>
   ```

   Numeric targets: `width`, `height`, `top`, `right`, `bottom`, `left`, `opacity`, `borderRadius`. Timing uses seconds. `ease` is `"linear"`, `"ease"`, `"easeIn"`, `"easeOut"`, `"easeInOut"`, or a cubic-bezier `[x1, y1, x2, y2]`.

   Set `initial={false}` to mount at the first `animate` target. A running animation can reverse or change target without a jump, because the next transition starts from the current visible value.

   Springs, keyframes, variants, exit transitions, and shared layout animations are not available yet.

2. **Playwright-like automation API** — mark elements with `testId`, then drive them from tests or from another process. Ordinary log lines are ignored.

   ```ts
   import { connectTest } from '@regenrek/gpuix-react/automation'

   const app = await connectTest(renderer)
   await app.getByTestId('inc').click()
   await app.getByText('Count: 1').waitFor()
   await app.getByTestId('composer').fill('hello gpuix')
   await app.getByTestId('composer').press('enter')
   await app.captureFrames('review/sidebar', [0, 150, 300])
   ```

   Locators: `getByTestId`, `getByText`, `getByType`. `click()` hits the center of the last painted bounds. `fill(text)` replaces the focused editor. `press('enter')` sends one key. `waitFor()` polls until exactly one match exists.

   `app.clock.pause()`, `set(ms)`, and `fastForward(ms)` freeze native motion time so CI can capture the same frames every run.

   A live app listens on stdin when stdin is a pipe, not a TTY. A terminal run is unchanged. `launch({ command, args })` pipes stdin and speaks SSE `data:` lines:

   ```ts
   import { launch } from '@regenrek/gpuix-react/automation'

   const app = await launch({ command: 'bun', args: ['examples/chat.tsx'] })
   await app.getByTestId('composer').fill('hello')
   await app.screenshot({ path: 'live.png' })
   await app.close()
   ```

## 0.3.0

1. **CSS grid on `div`** — `display: "grid"` plus `gridTemplateColumns` maps to GPUI's Taffy grid. Use `gridColumnMin: "max-content"` for tables so each column is as wide as its widest cell.

   ```tsx
   <div
     style={{
       display: 'grid',
       gridTemplateColumns: 3,
       gridColumnMin: 'max-content',
       rowGap: 1,
       columnGap: 1,
     }}
   >
     {cells}
   </div>
   ```

   `gridTemplateRows` and `gridRowMin` work the same on the other axis.

2. **Window chrome at open time** — `render()` now honors a transparent titlebar, traffic-light position, and a blurred or transparent window background. Traffic lights can sit in a sidebar. The native titlebar does not take a strip above the app.

   ```tsx
   import { render } from '@regenrek/gpuix-react'

   render(<App />, {
     title: 'Waku',
     width: 1180,
     height: 820,
     titlebarTransparent: true,
     windowBackground: 'blurred',
     trafficLightX: 16,
     trafficLightY: 17,
   })
   ```

   `windowBackground` is `"opaque"` (default), `"transparent"`, or `"blurred"`. The older `transparent: true` flag still maps to a transparent background when `windowBackground` is unset.

3. **`<diff>` flows with its parent** — it no longer owns a scroller unless you pass `scroll`. Nested scrolling is not supported in GPUI. A parent that already scrolls used to fight the inner `list()`. The default is now a column of rows, same as `<code>`.

   Use `maxLines` to keep a long patch short. Show more fires `onShowMore` with the hidden line count. Clear `maxLines` in that handler to reveal the rest.

   ```tsx
   const [open, setOpen] = useState(false)

   <diff
     patch={unifiedPatch}
     wordDiff
     maxLines={open ? undefined : 24}
     onShowMore={() => setOpen(true)}
   />
   ```

   Pass `scroll` and a bounded height only for a dedicated full-window viewer. That path still virtualizes with GPUI's `list()`.

4. **Debug frame overlay** — see draw time on a live window. The overlay paints after layout. It is not a React element.

   ```tsx
   import { render } from '@regenrek/gpuix-react'

   render(<App />, { title: 'My App', debugFrameOverlay: 'full' })
   ```

   Or call the renderer:

   ```ts
   renderer.setDebugFrameOverlay('full')
   renderer.cycleDebugFrameOverlay()
   renderer.resetDebugFrameOverlayStats()
   renderer.getDebugFrameOverlay()
   ```

   Modes are `hidden` (default), `minimal` (last draw time), and `full` (`CUR`, `1%`, `10%`, `MAX`, `FRAMES`). The readout is **draw time**, not FPS. `8.3 MS` is about 120 Hz.

5. **Quit when the last window closes** — on macOS the red traffic-light button used to destroy the window and leave the bun/Node process running. Closing the last window now quits AppKit. The next `tick()` returns `false`. `render()` exits the process, so the Dock icon goes away.

6. **Overlays block hits, and `pointerEvents` works** — a filled or absolutely positioned `div` now inserts a blocking hitbox. Clicks, hovers, and scroll no longer reach controls under a Select, Combobox, or any other card.

   Set `pointerEvents: "none"` to opt out. Set `pointerEvents: "auto"` to block even when the element has no fill.

7. **Opaque Select, Combobox, and Tooltip surfaces** — `FloatingLayer` now defaults to `backgroundColor: "#1A1A1A"` so window blur and page content do not show through the card. Pass your own `style.backgroundColor` to override.

8. **`<svg>` icons paint on the first frame** — file paths are read from disk. `data:image/svg+xml` URLs from Bun/Vitest `import … with { type: 'file' }` are percent-decoded. The icon paints with `svg().data(...)`.

9. **`bun --hot` remounts no longer paint a black window** — `render()` now unmounts the previous React root with `flushSync`, so the old tree is gone before the new one is created.

10. **Cmd+Delete and Cmd+Backspace in `<input>` and `<textarea>`** — on macOS these match the system text field. Cmd+Backspace deletes to the start of the line. Cmd+Delete deletes to the end of the line.

11. **Vertical wheel over `overflowX: "scroll"` stays on the parent** — GPUI remaps mouse-wheel Y onto overflow-x unless `restrict_scroll_to_axis` is set. A parent that contains `<code>` or a markdown table then used to jump on both axes. Trackpad X still pans the wide child.

    ```tsx
    <div style={{ overflowY: 'scroll' }}>
      <code code={wideSource} language="ts" />
    </div>
    ```

12. **Parent scroller takes the wheel over a filled in-flow `div`** — a `backgroundColor` used to insert `occlude()` (BlockMouse), so `<virtual-list>` never saw the wheel over text or a card. In-flow fills now use `block_mouse_except_scroll()`. Absolute, fixed, and `pointerEvents: "auto"` still steal the wheel.

13. **macOS scroll stays at the display rate on expensive frames** — `tick()` used to sleep a fixed 8ms after every pump. A 10ms scroll frame plus that sleep ran at about 55fps on a 120Hz display. The next pump now waits only the leftover budget.

14. **Faster first React mount** — `applyBatch` sends styles and custom props as JSON values instead of double-encoded strings. A 10,000-row list spent most of its mount time parsing escaped strings twice. Legacy string payloads still decode.

15. **Raw custom-prop values stay intact** — `setCustomProp` still treats the payload as a JSON string. After the batch started carrying objects, a raw `"top"` or `"true"` was parsed again and threw. `<anchored side="top">` never committed. The queue now uses `setCustomPropValue` for a raw JSON value.

## 0.2.0

1. **Selectable text everywhere, plus `<code>`, `<diff>` and `<markdown>`** — every string GPUIX paints can be selected with a drag and copied with Cmd+C. A drag can start in a plain `<text>` and end inside a code block; the selection spans both.

   ```tsx
   <div style={{ display: 'flex', flexDirection: 'column' }}>
     <text>drag from here</text>
     <code code={'and into this code block'} language="ts" />
   </div>
   ```

   Chrome opts out the same way CSS does, and it inherits:

   ```tsx
   <div style={{ userSelect: 'none' }}>
     <text>toolbar label, never selected</text>
   </div>
   ```

   Read it from the renderer with `renderer.getSelectedText()` and clear it with `renderer.clearSelection()`.

   **`<code>`** is a syntax-highlighted block. One row per line at an exact line height, so its height is known before highlighting runs and a late highlight never reflows it.

   ```tsx
   <code code={source} language="typescript" showLineNumbers />
   <code code={source} path="src/app.ts" />   {/* detect from the extension */}
   ```

   **`<diff>`** is a unified diff viewer virtualized with GPUI's `list()`, so a 2000-line patch paints only the rows on screen. Collapsing a file removes its rows rather than hiding them.

   ```tsx
   <diff
     patch={unifiedPatch}
     wordDiff
     collapsedPaths={['pnpm-lock.yaml']}
     onToggleFile={(e) => toggle(e.value)}
     onLineClick={(e) => console.log(e.oldLine, e.newLine, e.value)}
   />
   ```

   `wordDiff` highlights only the tokens that changed inside paired `+`/`-` lines.

   **`<markdown>`** is GitHub-flavoured markdown: headings, lists, tables, block quotes, fenced code, strikethrough, task lists, and autolinked bare URLs.

   ```tsx
   <markdown source={readme} onLinkClick={(e) => open(e.value)} />
   ```

   All three take the same `theme` prop. Fields layer on top of the built-in dark theme:

   ```tsx
   <code
     code={source}
     language="rust"
     theme={{
       appearance: 'light',
       accent: '#7c86ff',
       syntax: { keyword: '#f38ba8', string: '#a6e3a1' },
     }}
   />
   ```

   Bundled languages: Rust, TypeScript, TSX, JavaScript, JSX, Python, Go, JSON, Bash, TOML, YAML, Markdown, HTML, CSS, C.

   Row heights, gutter widths, paddings and the heading scale live in `theme.metrics`, so tuning the design is a React re-render and never a native rebuild.

   ```tsx
   <diff
     patch={patch}
     theme={{
       metrics: {
         diffLineHeight: 26,
         diffGutterWidth: 48,
         mdHeadingSizes: [24, 19, 16, 14],
       },
     }}
   />
   ```

   New style props: `userSelect` (`"text"` | `"none"`, inherited), `selectionColor`, and `lineHeight` is now applied.

   New test helpers: `renderer.getPaintedText()`, `renderer.dragSelect(x1, y1, x2, y2)`, and `renderer.getSyntaxCacheStats()`.

   Ported from [Comet](https://github.com/zeronsh/comet) (MIT). See `THIRD_PARTY_NOTICES.md`.

2. **Native `<input>` and `<textarea>`** — single-line and multiline editors backed by GPUI's platform input handler.

   ```tsx
   <textarea
     value={draft}
     minRows={1}
     maxRows={8}
     onChange={(event) => setDraft(event.value ?? '')}
     onSubmit={send}
   />
   ```

   Both support a native caret, mouse selection, IME composition, clipboard actions, undo/redo, caret movement and grapheme-safe deletion. `Enter` submits and `Shift+Enter` inserts a newline in a textarea.

3. **`render()` remounts React on the same native window** — a `bun --hot` save remounts the tree without creating a second window.

   ```tsx
   import { render } from '@regenrek/gpuix-react'

   function App() {
     return <div style={{ padding: 16 }}>hello</div>
   }

   render(<App />, { title: 'My App', width: 800, height: 600 })
   ```

   ```bash
   bun --hot app.tsx
   ```

   The first call creates the GPUI renderer, window, React root, and frame loop. Later calls reuse that host and remount the tree. `useState` resets. The native `.node` addon stays loaded.

   `createRoot`, `createRenderer`, and `startFrameLoop` still exist for tests and custom hosts. Pass `{ renderer }` into `render()` to drive the test renderer.

   React Refresh (keep hook state across saves) is not included.

4. **Headless Select, Combobox, and Tooltip** — unstyled primitives with the same compound composition used by shadcn. Import a namespace, wrap it in a local `components/ui/*.tsx`, and use those styled components in the app.

   ```tsx
   import * as SelectPrimitive from '@regenrek/gpuix-react/select'

   <SelectPrimitive.Root value={model} onValueChange={setModel}>
     <SelectPrimitive.Trigger>
       <SelectPrimitive.Value placeholder="Select a model" />
     </SelectPrimitive.Trigger>
     <SelectPrimitive.Content>
       <SelectPrimitive.Item value="sonnet">Sonnet</SelectPrimitive.Item>
     </SelectPrimitive.Content>
   </SelectPrimitive.Root>
   ```

   Dedicated entry points:

   | Import | Main parts |
   |---|---|
   | `@regenrek/gpuix-react/select` | `Root`, `Trigger`, `Value`, `Content`, `Item` |
   | `@regenrek/gpuix-react/combobox` | `Root`, `Input`, `Content`, `List`, `Item`, `Empty` |
   | `@regenrek/gpuix-react/tooltip` | `Provider`, `Root`, `Trigger`, `Content` |

   The barrel `@regenrek/gpuix-react` still exports the prefixed names (`Select`, `SelectTrigger`, and the rest).

   Each part accepts GPUIX styles, including state-based item style functions. Menus support native focus, keyboard navigation, outside-click dismissal, window-edge snapping, and click occlusion. Comboboxes use the native text input and rank prefix matches before substring matches.

5. **`<virtual-list>`** — long, variable-height React collections. GPUI builds and lays out only rows near the viewport while React and the native retained tree keep the complete collection.

   ```tsx
   <virtual-list
     alignment="bottom"
     followTail
     estimatedItemHeight={180}
     style={{ flexGrow: 1, minHeight: 0 }}
   >
     {messages.map((message) => (
       <Message key={message.id} message={message} />
     ))}
   </virtual-list>
   ```

   Rows can contain any GPUIX host or custom element. Appended rows preserve list measurements, changed rows are remeasured, and existing `scrollTo`, `scrollToItem`, and `getScrollOffset` methods work with virtual lists.

6. **Tintable local SVG icons** — `<svg>` uses GPUI's monochrome SVG renderer.

   ```tsx
   <svg
     src="/absolute/path/to/search.svg"
     style={{ width: 16, height: 16, color: '#b4b4b4' }}
   />
   ```

   `width` and `height` control layout. `color` controls the icon tint.

7. **`startFrameLoop()`** — stop burning CPU on idle apps. The old `setImmediate` loop spun at roughly 27,000 ticks per second and measured **73.5% CPU** on an idle counter. `startFrameLoop` paces at ~125fps (~1% CPU).

   ```tsx
   import { startFrameLoop } from '@regenrek/gpuix-react'

   startFrameLoop(renderer)
   ```

   ```tsx
   const loop = startFrameLoop(renderer, { frameMs: 16 })
   loop.stop()
   ```

   Each frame is scheduled only after the previous one finishes. Rendering is unchanged: one draw per React commit, and no draws at all while idle.

8. **Native GPUI platform** — Node applications use GPUI's native platform, window, renderer, and event pipeline on macOS, Windows, and Linux.

   On macOS, Node drives an embedded AppKit event pump from the pinned GPUIX fork on the process main thread. On Windows and Linux, GPUI runs its normal blocking event loop on a dedicated Rust UI thread while Node sends in-process render and window commands. Windows runtime validation is still pending.

9. **GPUI upgrade to zed `d5dc01f2`** — picks up several months of GPUI work, including `Application::run_embedded()`. GPUIX now holds the returned `ApplicationHandle` for the lifetime of the process.

   Scroll events can now report a cancelled phase. Previously a cancelled scroll gesture was reported to JS as `"ended"`.

   ```tsx
   <div
     style={{ overflow: 'scroll' }}
     onScroll={(e) => {
       if (e.touchPhase === 'cancelled') return
     }}
   />
   ```

   Building from source now requires **Rust 1.97.1**, pinned in `rust-toolchain.toml`. On macOS you also need the Metal compiler:

   ```bash
   xcodebuild -downloadComponent MetalToolchain
   ```

   Prebuilt binaries from npm are unaffected.

10. **Style props that were declared and dropped now work** — `<text>` takes the full style set (padding, width, backgroundColor, borderRadius, flex). `fontSize` works on `<div>` and custom elements. `textAlign`, `rowGap`, `columnGap`, and `lineHeight` are applied. `borderWidth: 0` can clear a border.

    ```tsx
    <text style={{ paddingLeft: 40, width: 300, backgroundColor: '#7c86ff', borderRadius: 12 }}>
      now works
    </text>
    ```

11. **`autoFocus` works and `<input>` is unstyled** — `autoFocus` was declared and dropped by the reconciler, so an `<input>` never held keyboard focus unless the user clicked it. It now works on every element type.

    ```tsx
    <input value={text} autoFocus onKeyDown={(e) => e.keyChar && setText(t => t + e.keyChar)} />
    ```

    `<input>` no longer hardcodes a background, border, or radius. Only the placeholder dims. Style the element or its wrapper:

    ```tsx
    <input
      value={text}
      style={{ backgroundColor: '#00000000', borderWidth: 0, color: '#ececec', fontSize: 15 }}
    />
    ```

    `<input>` is **controlled**: it paints `value` and reports keystrokes.

12. **Blinking caret** — the native input and textarea caret blinks every 500ms while focused and idle. Editing or moving the caret makes it immediately solid. Blurring the field stops its repaint timer.

    ```tsx
    <input theme={{ caret: '#22c55e' }} />
    ```

13. **Clipboard and natural scroll** — `Cmd+C` writes to the system clipboard via `arboard`. Wheel deltas keep the sign the OS already applied, so natural scrolling matches System Settings.

14. **React 19 JSX components** — the GPUIX JSX runtime accepts any valid `ReactNode` return type, so libraries such as `safe-mdx` can render parsed content into GPUIX host elements.

## 2026-03-02 23:30 UTC

- **Add hover/active pseudo-selector style support** — styles applied natively by GPUI with zero JS round-trips.
  - New `hover` and `active` keys in `StyleDesc` accept nested style objects: `style={{ backgroundColor: '#313244', hover: { backgroundColor: '#45475a' }, active: { backgroundColor: '#585b70' } }}`.
  - Rust `StyleDesc` (style.rs): added `hover: Option<Box<StyleDesc>>` and `active: Option<Box<StyleDesc>>` fields with serde support.
  - Renderer (renderer.rs): `build_div()` calls GPUI's native `.hover()` and `.active()` methods, passing the sub-styles through `apply_styles()` which works on `StyleRefinement` via the `Styled` trait.
  - TypeScript types (host.ts): `hover?` and `active?` typed as `Omit<StyleDesc, 'hover' | 'active'>` to prevent infinite nesting.
  - Added 7 tests validating hover-only, active-only, combined hover+active, empty hover, color-only hover, and hover alongside event handlers.

## 2026-03-02 16:50 UTC

- **Add GitHub Actions CI/CD pipeline** (`.github/workflows/ci.yml`) — builds native binaries for 4 targets (macOS arm64/x64, Linux x64/arm64), runs tests on macOS, and publishes to npm.
- Publish is version-gated: skips if the package.json version is already on npm. Bump version + push to main to release.
- Two packages published: `@regenrek/gpuix-native` (per-platform binaries via napi pre-publish) and `@regenrek/gpuix-react` (pure TypeScript).
- Generate `packages/native/npm/` per-platform package scaffolding (darwin-arm64, darwin-x64, linux-x64-gnu, linux-arm64-gnu).
- Add `build:release` script for Linux CI builds without test-support (gpui_macos is macOS-only).
- macOS builds include test-support by default so published binaries ship `TestGpuixRenderer` for user testing.
- Update `@regenrek/gpuix-react` dependency on `@regenrek/gpuix-native` from `workspace:*` to `workspace:^` for publishing.
- Add `publishConfig` to `@regenrek/gpuix-react` package.json.
- Document Cargo feature gate in Cargo.toml comments.

## 2026-03-02 16:32 UTC

- **Migrate `packages/native` from napi-rs v2 to v3** — prerequisite for CI/CD and per-platform npm publishing.
  - Bump `napi` crate from `2` to `3` and `napi-derive` from `2` to `3` in `Cargo.toml` (`napi-build` stays at `2`).
  - Bump `@napi-rs/cli` from `^2.18.0` to `^3.1.3` in `package.json`.
  - Switch napi config from v2 `triples` format (`name` + `triples.additional`) to v3 `targets` format (`binaryName` + `targets` array). Add `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` targets.
  - Change `prepublishOnly` script from `napi prepublish` to `napi pre-publish` (v3 hyphenated command).
  - Add `publishConfig` with `registry` and `access: "public"`.
  - Wrap `ThreadsafeFunction` in `Arc` in `GpuixRenderer` — napi v3 `ThreadsafeFunction` is `!Clone`, so `Arc` allows sharing it into the `GpuixView` closure from `&self` methods.
  - Generated `index.js` now uses v3's `requireNative()` function pattern (replaces v2's switch/case loader).
  - Generated `index.d.ts` now includes JSDoc comments from Rust `///` doc comments.
- All 105 tests pass.

## 2026-03-02 17:05 UTC

- Fix `fontWeight` to accept both string and number values — previously `fontWeight: 700` (number) would reject the entire mutation batch because the Rust deserializer only accepted strings. Now uses a `FontWeightValue` enum with `#[serde(untagged)]` that deserializes both `"bold"` (string) and `700` (number). Numeric values are clamped to 1–1000.
- All 105 tests pass.

## 2026-03-02 16:55 UTC

- Add `whiteSpace` support — `"nowrap"` prevents text wrapping (single line), `"normal"` enables wrapping (default). Applied in both `apply_styles()` and `build_text()` via GPUI's `.whitespace_nowrap()` / `.whitespace_normal()`.
- Add `textOverflow` support — `"ellipsis"` truncates long text with "..." at end, `"ellipsis-start"` truncates from the start. Applied in both `apply_styles()` and `build_text()` via GPUI's `.text_ellipsis()` / `.text_ellipsis_start()`.
- Add `lineClamp` support — limits text to N visible lines. Applied in both `apply_styles()` and `build_text()` via GPUI's `.line_clamp(n)`. Values < 1 are ignored.
- Update README: document all text style properties, add note about `white-space: pre` not being supported in GPUI with a workaround pattern (split `\n` + flex column + nowrap per line).
- Add 13 new tests in styles.test.tsx: whiteSpace nowrap/normal with visual comparison, textOverflow ellipsis/ellipsis-start with comparison, lineClamp at 1/2/3 lines with comparison, edge case lineClamp: 0, div-level inheritance for nowrap and lineClamp, pre-like behavior composite test, and short text no-truncation edge case.
- All 104 tests pass (82 existing + 22 in styles.test.tsx).

## 2026-03-02 16:30 UTC

- Wire up `alignSelf` in `apply_styles()` — field existed in StyleDesc but was never applied. Uses direct `el.style().align_self` field access since GPUI has no convenience methods. Supports center, start, end, stretch, baseline.
- Fix `flexGrow` and `flexShrink` to respect actual numeric values — previously `flexGrow: 0` and `flexGrow: 1` both produced the same result (hardcoded 1.0). Now sets `el.style().flex_grow = Some(value)` directly.
- Add `fontFamily` support — new field in `StyleDesc` (Rust + TS), applied in both `apply_styles()` and `build_text()` via GPUI's `.font_family()` method. Enables monospace fonts for code rendering.
- Wire up `fontWeight` in `apply_styles()` and `build_text()` — field existed in StyleDesc but was never applied. Parses CSS weight strings (named keywords like "bold"/"semibold" and numeric like "700") to `gpui::FontWeight`. Case-insensitive with hyphenated variants (extra-bold, semi-bold).
- Add `backgroundColor` support to `build_text()` — text elements can now have background colors via `.bg()` on the wrapping div. Enables word-level diff highlighting.
- Wire up `flexWrap` in `apply_styles()` — field existed in StyleDesc but was never applied. Maps "wrap" → `flex_wrap()`, "wrap-reverse" → `flex_wrap_reverse()`, "nowrap" → `flex_nowrap()`.
- Extract `parse_font_weight()` helper function to deduplicate font-weight parsing between `apply_styles()` and `build_text()`.
- Add `styles.test.tsx` with 9 end-to-end tests covering all new features with Metal GPU screenshots: alignSelf stretch, flexShrink 0, flexGrow values, fontFamily (Menlo/Courier vs default), fontWeight (bold/light/normal), text backgroundColor, flexWrap, and a composite diff-viewer row test.
- All 91 tests pass.

## 2026-03-02 14:54 UTC

- Add test proving React refs expose the element's numeric ID (`ref.current.id`) for use with programmatic scroll API
- Remove dead `id?: string` prop from `Props` type — it was never wired to anything
- Add scroll usage docs to README: `overflow: "scroll"` example, per-axis scrolling, and programmatic scroll via refs
- Comment Props type to document that element IDs come from refs, not a user prop

## 2026-03-02 14:42 UTC

- Add scrollable container support — `overflow: "scroll"`, `overflowX: "scroll"`, `overflowY: "scroll"` now create native GPUI scrollable divs
- GPUI handles scroll physics automatically: scroll wheel events update a persistent `ScrollHandle` offset, content is clipped and translated, offset is clamped to valid bounds
- `ScrollHandle` persists across frames in `GpuixView::scroll_handles` (keyed by element ID), same lifecycle pattern as `focus_handles`
- Add per-axis overflow hidden support: `overflowX: "hidden"` and `overflowY: "hidden"` now map to `overflow_x_hidden()` / `overflow_y_hidden()`
- Add programmatic scroll API via napi: `scrollTo(elementId, x, y)`, `scrollToItem(elementId, index)`, `getScrollOffset(elementId)` on both `GpuixRenderer` and `TestGpuixRenderer`
- Production renderer syncs scroll handles to a thread_local (`SCROLL_HANDLES`) after each render so napi methods can access them without an App context
- TestRenderer exposes `scrollTo()`, `scrollToItem()`, `getScrollOffset()` wrapper methods
- NativeRenderer interface updated with optional scroll methods
- Add 6 new end-to-end scroll tests: basic scroll, overflow-y only, programmatic scrollTo, scrollToItem, screenshot regression (before/after scroll), and onScroll event + overflow scroll combo
- All 80 tests pass

## 2026-03-01 20:45 UTC

- Remove JS shadow tree from TestRenderer — all element state now lives exclusively in Rust's RetainedTree, queried via napi
- TestRenderer inspection methods (findByType, getAllText, toJSON, getRoot, getElement, findByText) now query the native TestGpuixRenderer instead of maintaining a parallel JS element map
- Add `getRootId()` napi method to TestGpuixRenderer for root element queries
- Add `customProps` to `getTreeJson()` output so test inspection can see custom element props (used by img/input tests)
- TestRenderer constructor now requires native renderer (throws if not available); tests already skip via `hasNativeTestRenderer`
- Net ~220 lines of redundant JS state management code removed
- All 68 tests pass — zero test file changes needed

## 2026-03-01 20:30 UTC

- Add FFI mutation batching — all React reconciler mutations per commit are now buffered JS-side and sent to Rust in a single `applyBatch()` napi call instead of N individual FFI calls
- Add `apply_batch(json)` to both `GpuixRenderer` and `TestGpuixRenderer` (Rust) — parses a JSON array of string-named mutation tuples `["methodName", ...args]` and applies them under a single mutex lock
- Atomic two-phase Rust processing: `parse_batch_ops()` validates all ops into typed `BatchOp` enum before any tree mutation; malformed batch → error with tree unchanged
- Add Proxy-based `wrapWithBatching()` (`batch-renderer.ts`) — auto-captures any NativeRenderer method call as `[name, ...args]`; adding new methods requires zero changes to the batching layer
- TestRenderer uses `_skipNative` flag + dynamic dispatch for `applyBatch()` replay — also zero changes needed when adding new methods
- Wire `wrapWithBatching()` into both `createRoot()` and `createTestRoot()` — batching is automatic when the renderer supports `applyBatch()`
- Backward compatible: individual mutation methods remain available; batching is opt-in via `applyBatch` presence
- All 68 existing tests pass through the batched path

## 2026-03-01 19:07 UTC

- Add native `<img>` custom element backed by `gpui::img(PathBuf)` with `src` and `objectFit` custom props and fallback rendering states for missing/failed sources
- Register image factory in the custom element registry and expose `ImgProps` in React JSX runtime/dev-runtime type surfaces
- Add new end-to-end `img.test.tsx` suite including screenshot regression that captures before/after PNGs when image `src` is set

## 2026-03-01 18:52 UTC

- Add new `<anchored>` custom element with GPUI `anchored()` positioning props (`x`/`y`, `position`, `anchor`, `snapToWindow`, `snapMargin`) and optional deferred overlay rendering (`deferred`, `priority`)
- Extend custom element render context to pass built child elements so custom primitives can wrap and position nested React content
- Register `anchored` in the default custom element registry and expose it in React intrinsic types/component map
- Add end-to-end anchored deferred dialog overlay test (open, inside click stays open, outside click closes)

## 2026-03-01 18:47 UTC

- Add dialog overlay screenshot regression test that captures before/after PNGs and asserts visual output changes when opening the dialog

## 2026-03-01 18:45 UTC

- Add absolute positioning support in native style mapping (`position`, `top`, `right`, `bottom`, `left`) so React styles place elements out of flow like dialogs/tooltips
- Add end-to-end dialog overlay test: click button opens tooltip-like dialog content, inside click keeps it open, outside click closes via `onMouseDownOutside`

## 2026-03-01 18:35 UTC

- Add polymorphic custom element trait infrastructure (`CustomElement`, `CustomElementFactory`, `CustomElementRegistry`)
- Implement `<input>` as first custom element with value/placeholder/readOnly props and keyboard event handling
- Add `custom_props` field to `RetainedElement` for storing non-style/non-event props on custom elements
- Add `setCustomProp`/`getCustomProp` napi methods on both `GpuixRenderer` and `TestGpuixRenderer`
- Add custom prop forwarding in React reconciler (`host-config.ts`) — automatically syncs non-reserved props for non-div/text elements
- Add `InputProps` type and `input` to JSX IntrinsicElements
- Add 6 end-to-end tests: input rendering, keyboard typing (controlled component), backspace, screenshot before/after, tree structure
- Fix jsx-dev-runtime.js to export `jsxDEV` for React 19 compatibility with vitest (was breaking all tests)
- All 27 tests pass (6 new input + 21 existing events)

## 2026-03-01 17:42 UTC

- Fix custom element lifecycle cleanup by pruning/destroying stale trait instances when IDs disappear from the retained tree
- Fix stale custom prop state by resetting missing known props to `null` each frame via `supported_props()` synchronization
- Apply retained `style` to custom elements through `CustomRenderContext` so `<input style={...}>` affects native layout/hit-testing
- Filter custom element event wiring to declared `supported_events()` only
- Harden React custom prop forwarding with safe JSON serialization fallback (`null` on unsupported/circular values)
- Expand input end-to-end coverage with `readOnly` removal regression test and style-based click hit-test assertion

## 2026-03-01 17:15 UTC

- Rewrite README to reflect current mutation-based architecture (was describing old JSON tree approach)
- Replace "description-based renderer" language with "mutation-based protocol over napi-rs FFI"
- Add architecture diagram showing individual napi calls (createElement, appendChild, setStyle, commitMutations)
- Add Mutation API section documenting the full NativeRenderer interface
- Add Event Flow section with pipeline diagram (GPUI → Rust closure → ThreadsafeFunction → JS event registry → React handler)
- Add detailed events table with payload fields for each event type
- Add Testing section covering TestGpuixRenderer (GPU-backed Metal tests, screenshot capture, native event simulation)
- Update status checklist: mark keyboard events, focus/blur, scroll, click-outside, and test renderer as completed
- Update usage example to use createRenderer() instead of raw GpuixRenderer constructor

## 2026-03-01 16:48 UTC

- Center screenshot probe cards in the visual renderer tests so captured frames represent realistic composition instead of top-left anchored blocks
- Improve screenshot test visuals with richer card styling (rounded surfaces, palette contrast, readable text hierarchy)
- Keep visual assertions unchanged (before/after PNG difference) while moving click/hover simulation coordinates to centered card hit zones

## 2026-03-01 16:35 UTC

- Expand visual screenshot coverage with additional end-to-end tests for `click`, `keyDown`, and `mouseEnter`-driven hover state changes
- Add shared screenshot assertion helper in `events.test.tsx` to enforce non-empty PNG output and before/after image differences

## 2026-03-01 16:20 UTC

- Fix `build_text` to render child text elements recursively instead of dropping nested text nodes
- Improve screenshot reliability by forcing `window.refresh()` before `capture_screenshot()` in the native test renderer
- Strengthen screenshot integration test to assert visual output changes (compare PNG bytes before vs after interaction)
- Update screenshot test fixture to use a high-contrast background toggle so black-frame regressions are obvious

## 2026-03-01 15:40 UTC

- Switch TestGpuixRenderer from `TestAppContext` (no GPU) to `VisualTestAppContext` (real Metal rendering on macOS)
- Add `gpui_macos` dependency for `MacPlatform` — provides real Metal GPU rendering in test windows
- Replace raw `VisualTestContext` pointer with `VisualTestAppContext` + `AnyWindowHandle` in thread_local storage
- Add `capture_screenshot(path)` napi method — renders via Metal, reads back pixels, saves as PNG
- Add `captureScreenshot(path)` JS wrapper to `TestRenderer`
- Add screenshot integration test (renders counter, clicks, captures before/after PNGs)
- Gate `test_renderer` module on `#[cfg(all(feature = "test-support", target_os = "macos"))]`
- All 19 tests pass (18 existing event/tree tests + 1 new screenshot test)

## 2026-03-01 15:24 UTC

- Fix missing text in macOS visual screenshots by enabling `gpui_macos/font-kit` under `test-support`
- Keep `VisualTestAppContext` on real `MacTextSystem` instead of fallback `NoopTextSystem`, restoring glyph rasterization in `capture_screenshot()`
- Validate with an example-like counter render: text labels (`0/1`, `+`, `-`, `Reset`) now appear correctly in captured PNGs

## 2026-03-01 12:50 UTC

- Add plan for GPU-backed test renderer with screenshot support (`docs/visual-screenshot-plan.md`)
- Plan uses GPUI's `VisualTestAppContext` + Metal rendering on macOS (Oracle-reviewed, original headless wgpu approach rejected due to `WgpuRenderer` being surface-bound)

## 2026-03-01 12:25 UTC

- Add changelog requirement to AGENTS.md
- Document auto-generated napi-rs files in AGENTS.md (`index.d.ts`, `index.js`, `*.node`)

## 2026-03-01 12:00 UTC

- Add `simulate_key_down(keystroke, is_held?)` and `simulate_key_up(keystroke)` to Rust TestGpuixRenderer for fine-grained key event testing
- Extend `simulate_mouse_move(x, y, pressed_button?)` to accept optional pressed button for drag simulation
- Add `nativeSimulateKeyDown`, `nativeSimulateKeyUp` JS wrappers to TestRenderer
- Update `nativeSimulateMouseMove` to pass pressed button through to native
- Restore dropped tests: keyUp state update, keyDown+keyUp sequence, mouse button mapping (left/right/middle), drag pressedButton
- Tighten weak assertions: scroll checks exact deltaX/deltaY/touchPhase, mouseMove checks exact x/y
- Fix stale "mock-only mode" comment in testing.ts

## 2026-03-01 11:45 UTC

- Migrate all event tests from JS-only simulation to native GPUI end-to-end simulation
- Add `simulate_mouse_down(x, y, button)` and `simulate_mouse_up(x, y, button)` to Rust TestGpuixRenderer
- Add `nativeSimulateMouseDown` and `nativeSimulateMouseUp` JS wrappers to TestRenderer
- Remove all 10 JS-only simulation methods from TestRenderer (`simulateEvent`, `simulateClick`, `simulateKeyDown`, `simulateKeyUp`, `simulateMouseEnter`, `simulateMouseLeave`, `simulateMouseDown`, `simulateMouseUp`, `simulateMouseMove`, `simulateScroll`)
- Rewrite all tests to use coordinate-based native GPUI simulation with explicit element sizes
- Change key names from `"arrowDown"`/`"arrowUp"` to GPUI names `"down"`/`"up"`
