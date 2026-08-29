---
'@regenrek/gpuix-native': minor
'@regenrek/gpuix-react': minor
---

Add `SplitView`, a generic two-pane native GPUI layout. It reserves its divider
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
