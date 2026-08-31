---
"@regenrek/gpuix-react": patch
---

Destroy conditionally removed host subtrees during React's mutation phase so native custom elements tear down before the commit batch flushes.
