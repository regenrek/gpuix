---
'@regenrek/gpuix-native': patch
---

Speed up long virtual lists and automation locators.

A 5,000-row chat used to rebuild virtual-list focus maps on every GPUI frame, even when the row ids had not changed. Sidebar motion and caret blink then paid that cost on every tick. Unchanged lists now return before that work.

`getAutomationTree()` also stops serializing style, events, and custom props. Locators only need `id`, `type`, `testId`, `text`, and bounds. On a 5k-row tree that dropped tree JSON from about 110ms to about 22ms, so `getByTestId().click()` is no longer dominated by encoding unused style maps.
