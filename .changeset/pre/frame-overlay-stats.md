---
'@regenrek/gpuix-native': minor
'@regenrek/gpuix-react': minor
---

Add `getDebugFrameOverlayStats()` so tests and apps can read the same draw times the on-screen overlay shows.

```ts
renderer.resetDebugFrameOverlayStats()
// ... scroll or click ...
const stats = renderer.getDebugFrameOverlayStats()
// stats.currentMs, stats.p90Ms, stats.p99Ms, stats.maxMs, stats.frames, stats.samples
```

`p90Ms` is the overlay **10%** line. `p99Ms` is the **1%** line. Those are the slow tail, not the fast frames.

The chat example uses this in `examples/chat.perf.test.tsx` to catch mount, wheel, and sidebar regressions.
