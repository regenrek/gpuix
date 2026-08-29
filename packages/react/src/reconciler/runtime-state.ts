import type {
  Container,
  ElementIdAllocator,
  NativeRenderer,
} from "../types/host.js"

const RUNTIME_STATE_KEY = Symbol.for("@regenrek/gpuix-react/runtime-state")

interface RuntimeState {
  containersByRenderer: WeakMap<NativeRenderer, Container>
  idAllocatorsByRenderer: WeakMap<NativeRenderer, ElementIdAllocator>
}

function createRuntimeState(): RuntimeState {
  return {
    containersByRenderer: new WeakMap(),
    idAllocatorsByRenderer: new WeakMap(),
  }
}

/**
 * State shared by every evaluated copy of the reconciler modules.
 *
 * Bun hot reload evaluates fresh module instances while the native renderer
 * and its event callback remain alive. Keeping these renderer-indexed maps on
 * globalThis lets that callback reach the newly mounted React root and keeps
 * element ids monotonic across reloads.
 */
export function runtimeState(): RuntimeState {
  const existing = Reflect.get(globalThis, RUNTIME_STATE_KEY) as
    | RuntimeState
    | undefined
  if (existing) return existing

  const created = createRuntimeState()
  Reflect.set(globalThis, RUNTIME_STATE_KEY, created)
  return created
}

export function idAllocatorForRenderer(
  renderer: NativeRenderer
): ElementIdAllocator {
  const allocators = runtimeState().idAllocatorsByRenderer
  let allocator = allocators.get(renderer)
  if (!allocator) {
    allocator = { nextElementId: 0 }
    allocators.set(renderer, allocator)
  }
  return allocator
}
