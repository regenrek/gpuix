---
"@regenrek/gpuix-react": patch
---

Dispose each GPU-backed test renderer's native state after React test-root teardown so consecutive and concurrent roots remain isolated.
