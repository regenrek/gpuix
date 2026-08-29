---
'@regenrek/gpuix-native': minor
'@regenrek/gpuix-react': minor
---

Add a windowed `VirtualList` so long transcripts do not create every React row at mount.

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
