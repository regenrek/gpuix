/** macOS Appshot proof surface. Run with `bun --hot appshot.tsx`. */
import React, { useEffect, useRef, useState } from "react"
import { render, useGpuix } from "@regenrek/gpuix-react"

const button = { padding: 10, backgroundColor: "#384a8a", color: "#ffffff", borderRadius: 6, cursor: "pointer" } as const

function AppshotExample() {
  const { renderer } = useGpuix()
  const [status, setStatus] = useState("Check screen-capture permission, then choose a window.")
  const [selection, setSelection] = useState<string | null>(null)
  const [preview, setPreview] = useState<string | null>(null)
  const previewRef = useRef<string | null>(null)

  const disposePreview = () => {
    const handle = previewRef.current
    if (handle) renderer.disposeAppshotPreview(handle)
    previewRef.current = null
    setPreview(null)
  }

  useEffect(() => () => {
    const handle = previewRef.current
    if (handle) renderer.disposeAppshotPreview(handle)
    previewRef.current = null
  }, [renderer])

  const previewPng = async (png: Buffer) => {
    disposePreview()
    const handle = renderer.createAppshotPreview(png)
    previewRef.current = handle
    setPreview(handle)
  }

  return <div style={{ display: "flex", flexDirection: "column", gap: 12, padding: 24, width: 680, height: 560, backgroundColor: "#171923" }}>
    <text style={{ fontSize: 20, color: "#eef1ff" }}>Appshot macOS proof surface</text>
    <text style={{ color: "#b7bdd8" }}>{status}</text>
    <div style={{ display: "flex", gap: 8 }}>
      <div onClick={() => setStatus(JSON.stringify(renderer.preflightAppshotPermission()))} style={button}>Preflight permission</div>
      <div onClick={() => setStatus(JSON.stringify(renderer.requestAppshotPermission()))} style={button}>Request permission</div>
      <div onClick={async () => { const result = await renderer.selectAppshotWindow(); setSelection(result.handle ?? null); setStatus(result.status) }} style={button}>Pick window</div>
      <div onClick={async () => { if (!selection) return setStatus("Pick a window first"); await previewPng(await renderer.captureAppshotWindow(selection)); setSelection(null); setStatus("Selected capture previewed") }} style={button}>Capture selected</div>
    </div>
    <div style={{ display: "flex", gap: 8 }}>
      <div onClick={async () => { await previewPng(await renderer.captureFrontmostAppshot()); setStatus("Frontmost capture previewed") }} style={button}>Capture frontmost</div>
      <div onClick={disposePreview} style={button}>Dispose preview</div>
    </div>
    {preview ? <img appshotPreviewHandle={preview} objectFit="contain" style={{ width: 620, height: 340, backgroundColor: "#090a10" }} /> : <div style={{ height: 340, backgroundColor: "#090a10", color: "#77809f", padding: 16 }}>No preview. Native bytes are not retained in React.</div>}
  </div>
}

render(<AppshotExample />, { title: "GPUIX Appshot", width: 730, height: 620 })
