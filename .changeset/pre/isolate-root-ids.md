---
"@regenrek/gpuix-react": patch
---

Give each React root its own event handler map. Two `createTestRoot()` trees can both start at id `1` without overwriting each other's handlers. `resetIdCounter()` is gone.

A remount on the same native renderer keeps allocating new ids. A late event from the old tree cannot hit a new handler that reused id `1`.

`handleGpuixEvent` now needs the renderer that produced the event:

```ts
handleGpuixEvent(event, renderer)
```
