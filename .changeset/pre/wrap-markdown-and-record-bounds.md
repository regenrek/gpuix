---
'@regenrek/gpuix-native': patch
'@regenrek/gpuix-react': patch
---

Make native `<markdown>` wrap in flex columns, and record painted bounds on `<markdown>`, `<code>`, and `<diff>`.

A markdown node in a flex row used to keep its max-content width, so a long paragraph or list item could blow past the parent. The root and each text block now shrink with `min-width: 0`, the same rule list items already used.

```tsx
<div style={{ display: 'flex', flexDirection: 'row', width: 280 }}>
  <div style={{ width: 40, flexShrink: 0 }} />
  <markdown
    source="- a long sentence that must wrap in the remaining column"
    style={{ flexGrow: 1 }}
  />
</div>
```

Fenced code inside `<markdown>` now matches `<code>`: long lines scroll on X and leave the vertical wheel on the parent. Before this they clipped at the rounded card.

`getElementBounds` and automation locators also work on those three elements, including an empty `<markdown source="" />`. They never painted a bounds tracker, so a `testId` on `<markdown>` returned null.

`TestRenderer.findByTestId()` looks up that `testId` from the retained tree.
