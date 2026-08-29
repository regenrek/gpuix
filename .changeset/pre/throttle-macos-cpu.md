---
'@regenrek/gpuix-react': minor
---

Add `THROTTLE` for macOS CPU clamps on profile runs.

`THROTTLE=utility` restarts the process under `taskpolicy -c utility`. That pins work to E-cores. Use it as an M1/M2 Air CPU proxy. `background` and `maintenance` are slower.

```bash
THROTTLE=utility bun run test chat.perf.test.tsx
THROTTLE=utility bun --hot chat.tsx
```

This is not Chrome 6x. GPU and RAM stay on the host machine. Do not set `THROTTLE` in CI.
