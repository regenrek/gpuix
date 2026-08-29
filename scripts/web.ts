/** Build the browser Wasm target and serve the GPUI canvas with isolation headers. */

import { spawn } from "node:child_process"
import fs from "node:fs"
import path from "node:path"
import { fileURLToPath } from "node:url"

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..")
const NATIVE = path.join(ROOT, "packages", "native")
const EXAMPLE_OUTPUT = path.join(ROOT, "examples", "web-dist")
const PACKAGE_OUTPUT = path.join(NATIVE, "wasm")
const WASM = path.join(NATIVE, "target", "wasm32-unknown-unknown", "release", "gpuix_native.wasm")
const WEB_RUST_TOOLCHAIN = "nightly-2026-08-28"

function run({ command, args, cwd }: { command: string; args: string[]; cwd: string }): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, stdio: "inherit" })
    child.on("error", reject)
    child.on("exit", (code) => {
      if (code === 0) resolve()
      else reject(new Error(`${command} exited with code ${code ?? 1}`))
    })
  })
}

async function main() {
  console.log("web: building gpuix-native for wasm32-unknown-unknown")
  await run({
    command: "cargo",
    args: [
      `+${WEB_RUST_TOOLCHAIN}`,
      "build",
      "--target",
      "wasm32-unknown-unknown",
      "--no-default-features",
      "--release",
      "--lib",
    ],
    cwd: NATIVE,
  })

  fs.mkdirSync(PACKAGE_OUTPUT, { recursive: true })
  console.log("web: generating the @regenrek/gpuix-native browser loader")
  await run({
    command: "wasm-bindgen",
    args: [WASM, "--target", "web", "--out-dir", PACKAGE_OUTPUT, "--out-name", "gpuix-web"],
    cwd: NATIVE,
  })

  if (process.argv.includes("--build-only")) {
    console.log(`web: generated ${path.relative(ROOT, PACKAGE_OUTPUT)}`)
    return
  }

  console.log("web: building @regenrek/gpuix-react")
  await run({ command: "bun", args: ["run", "build"], cwd: path.join(ROOT, "packages", "react") })

  console.log("web: bundling the ChatGPT example")
  fs.rmSync(EXAMPLE_OUTPUT, { recursive: true, force: true })
  fs.mkdirSync(EXAMPLE_OUTPUT, { recursive: true })
  const bundle = await Bun.build({
    throw: false,
    entrypoints: [path.join(ROOT, "examples", "web-chat.tsx")],
    outdir: EXAMPLE_OUTPUT,
    target: "browser",
    format: "esm",
    naming: "chat.js",
    plugins: [
      {
        name: "gpuix-web",
        setup(build) {
          build.onResolve({ filter: /^node:url$/ }, () => ({ path: "url", namespace: "gpuix-web" }))
          build.onLoad({ filter: /^url$/, namespace: "gpuix-web" }, () => ({
            contents: "export const fileURLToPath = (url) => url.href",
            loader: "js",
          }))
        },
      },
    ],
  })
  if (!bundle.success) {
    for (const log of bundle.logs) console.error(log)
    throw new Error("browser bundle failed")
  }

  const port = Number(process.env.PORT || 4173)
  const files = new Map([
    ["/", path.join(ROOT, "examples", "web.html")],
    ["/browser.mjs", path.join(NATIVE, "browser.mjs")],
    ["/wasm/gpuix-web.js", path.join(PACKAGE_OUTPUT, "gpuix-web.js")],
    ["/wasm/gpuix-web_bg.wasm", path.join(PACKAGE_OUTPUT, "gpuix-web_bg.wasm")],
    ["/gpuix-web_bg.wasm", path.join(PACKAGE_OUTPUT, "gpuix-web_bg.wasm")],
    ["/chat.js", path.join(EXAMPLE_OUTPUT, "chat.js")],
  ])
  const isolationHeaders = {
    "Cross-Origin-Embedder-Policy": "require-corp",
    "Cross-Origin-Opener-Policy": "same-origin",
  }
  const server = Bun.serve({
    port,
    fetch(request) {
      const pathname = new URL(request.url).pathname
      if (pathname === "/favicon.ico") {
        return new Response(null, { status: 204, headers: isolationHeaders })
      }
      const asset = pathname.startsWith("/assets/")
        ? path.join(ROOT, "examples", pathname.slice(1))
        : undefined
      const bundledWasm = /^\/gpuix-web_bg-[\w-]+\.wasm$/.test(pathname)
        ? path.join(EXAMPLE_OUTPUT, pathname.slice(1))
        : undefined
      const file = files.get(pathname) ?? asset ?? bundledWasm
      if (!file) return new Response("Not found", { status: 404 })
      return new Response(Bun.file(file), {
        headers: {
          ...isolationHeaders,
          "Content-Type": pathname.endsWith(".wasm")
            ? "application/wasm"
            : pathname.endsWith(".js") || pathname.endsWith(".mjs")
              ? "text/javascript"
              : "text/html",
        },
      })
    },
  })
  console.log(`web: http://localhost:${server.port}`)
}

main().catch((error) => {
  console.error(`web: ${error instanceof Error ? error.message : String(error)}`)
  process.exit(1)
})
