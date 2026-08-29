---
"@regenrek/gpuix-react": patch
---

Keep abandoned concurrent renders out of the native mutation queue.

React may throw away a Suspense render. GPUIX now waits until commit before it creates native elements, so fallback text paints and abandoned text does not.

Unchanged click handlers also stay registered across rerenders. GPUIX no longer clears the whole handler map before every update.
