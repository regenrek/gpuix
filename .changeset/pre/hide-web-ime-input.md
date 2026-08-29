---
'@regenrek/gpuix-native': patch
'@regenrek/gpuix-react': patch
---

Hide GPUI Web's IME text bridge so browser hosts no longer show a native input at the top of the page.

GPUI Web keeps a focused DOM `<input data-gpui-input>` for clipboard, keyboard, and composition. It sat at `top: 0` with only a 1px opacity-0 box, so host `input` CSS could unhide it. The control now uses important inline hide styles, `clip-path`, and `autocomplete="off"`. It stays focusable for IME.

```html
<!-- still present for IME, no longer painted -->
<input data-gpui-input autocomplete="off" />
```
