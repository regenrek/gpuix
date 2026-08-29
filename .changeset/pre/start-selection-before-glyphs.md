---
'@regenrek/gpuix-native': patch
---

Start a text selection from the empty space before the glyphs.

A press in parent padding, a code gutter, or the empty start of a line now clamps to the nearest text on that row. Before this, the mouse-down had to land inside the tight `TextLayout` box, so a drag that started just before the first character selected nothing.

```
  [padding] hello world
      ^
      press here, drag right  →  "hello world"
```

A press above or below every line still does not start a selection. That keeps a composer or titlebar from claiming the nearest paragraph. A click without movement still selects nothing.

`userSelect: "none"` now also blocks the start. A sidebar or other chrome on the same row as a paragraph will not start a selection on that paragraph. Native `<input>` and `<textarea>` own their own selection and do the same.
