/// Test utilities shared across GPUIX test files.

import fs from "fs"
import path from "path"
import { fileURLToPath } from "url"
import { expect } from "vitest"

export const isCI = !!process.env.CI

/** Where visual tests write their PNGs. Kept in the repo (gitignored) rather
 *  than /tmp so the output can actually be looked at after a run. */
export const SHOTS_DIR = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../screenshots"
)

/** Assert two screenshot PNGs exist, are non-empty, and differ.
 *  Skipped on CI — Metal on macOS VMs doesn't repaint between captures,
 *  producing byte-identical screenshots regardless of state changes. */
export function expectScreenshotsDiffer(beforePath: string, afterPath: string) {
  expect(fs.existsSync(beforePath)).toBe(true)
  expect(fs.existsSync(afterPath)).toBe(true)
  expect(fs.statSync(beforePath).size).toBeGreaterThan(0)
  expect(fs.statSync(afterPath).size).toBeGreaterThan(0)

  if (isCI) return

  const before = fs.readFileSync(beforePath)
  const after = fs.readFileSync(afterPath)
  expect(before.equals(after)).toBe(false)
}

export function expectScreenshotsEqual(leftPath: string, rightPath: string) {
  expect(fs.existsSync(leftPath)).toBe(true)
  expect(fs.existsSync(rightPath)).toBe(true)
  const left = fs.readFileSync(leftPath)
  const right = fs.readFileSync(rightPath)
  expect(left.equals(right)).toBe(true)
}
