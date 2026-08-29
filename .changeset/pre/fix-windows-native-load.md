---
'@regenrek/gpuix-native': patch
---

Fix Windows x64 native binding failing to load with `ERR_DLOPEN_FAILED`.

`require('@regenrek/gpuix-native')` no longer dies with `LoadLibrary failed: The specified procedure could not be found`. The published `.node` was statically importing `TaskDialogIndirect` from comctl32 v6 and `u_strlen` from `icuuc.dll`. Node and Bun do not activate comctl32 v6, so Windows resolved the old comctl32 and `LoadLibrary` failed before any JS ran.

```bash
bun -e "require('@regenrek/gpuix-native'); console.log('OK')"
```

Fixes #1
Closes #2
