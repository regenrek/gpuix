import { applyMacCpuThrottleFromEnv } from '@regenrek/gpuix-react'
import { defineConfig } from 'vitest/config'

applyMacCpuThrottleFromEnv()

export default defineConfig({})
