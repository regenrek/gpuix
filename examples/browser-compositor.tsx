/**
 * macOS-only proof for GPUI Base / WKWebView / GPUI Overlay composition.
 * It intentionally uses local data URLs so the proof has no network dependency.
 */
import React, { createElement, useState } from "react"
import { render, SplitView } from "@regenrek/gpuix-react"

const BROWSER_A_PROFILE = "1bf766d4-9632-4292-b2d4-6d7058cd78af"
const BROWSER_B_PROFILE = "a4eb96bc-1f22-471a-b9f8-bc7c2f7633a2"

const page = (label: string) =>
  `data:text/html;charset=utf-8,${encodeURIComponent(`<!doctype html><html><head><meta charset="utf-8"><title>${label}</title><style>html,body{background:#f8fbff;color:#182235}body{font:16px -apple-system;padding:24px}input{display:block;margin-top:16px;padding:10px;width:80%;background:#fff;color:#182235;border:1px solid #6e7d96}</style></head><body><h1>${label}</h1><p>Native WKWebView inside GPUIX.</p><input value="select and copy me"></body></html>`)}`

type BrowserProps = {
  label: string
  profileId: string
  visible?: boolean
  url: string
  reloadRequestId?: string
  clearDataRequestId?: string
  onEvent: (message: string) => void
}

function Browser({ label, profileId, visible = true, url, reloadRequestId, clearDataRequestId, onEvent }: BrowserProps) {
  return createElement("browser-surface", {
    url,
    profileId,
    visible,
    reloadRequestId,
    clearDataRequestId,
    style: { width: "100%", height: "100%", borderRadius: 12 },
    onBrowserNavigation: (event: { browserUrl: string; browserCanGoBack: boolean; browserCanGoForward: boolean }) =>
      onEvent(`${label}: navigated (${event.browserCanGoBack ? "back" : "start"}/${event.browserCanGoForward ? "forward" : "end"}) ${event.browserUrl}`),
    onBrowserLoading: (event: { browserIsLoading: boolean; browserUrl: string }) =>
      onEvent(`${label}: ${event.browserIsLoading ? "loading" : "loaded"} ${event.browserUrl}`),
    onBrowserDownload: (event: { browserDownloadId: string; browserSuggestedFilename: string }) =>
      onEvent(`${label}: download ${event.browserDownloadId} ${event.browserSuggestedFilename}`),
    onBrowserDataCleared: (event: { browserProfileId: string; browserRequestId: string }) =>
      onEvent(`${label}: cleared ${event.browserProfileId} (${event.browserRequestId})`),
  })
}

function App() {
  const [menu, setMenu] = useState(false)
  const [showSecond, setShowSecond] = useState(true)
  const [browserAUrl, setBrowserAUrl] = useState(page("Browser A"))
  const [reloadA, setReloadA] = useState(0)
  const [clearA, setClearA] = useState(0)
  const [events, setEvents] = useState<string[]>([])
  const record = (event: string) => setEvents((current) => [event, ...current].slice(0, 4))
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", backgroundColor: "#0b1020", color: "#e8eefc", padding: 12, gap: 8 }}>
      <div style={{ display: "flex", gap: 8, height: 38 }}>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setMenu(!menu)}>Toggle GPUI menu</div>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setShowSecond(!showSecond)}>Hide/show Browser B</div>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setBrowserAUrl("https://example.com")}>Navigate A remotely</div>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setReloadA((value) => value + 1)}>Reload A</div>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setClearA((value) => value + 1)}>Clear A data</div>
        <div style={{ width: 48, flexShrink: 0, padding: 9, backgroundColor: menu ? "#1c6b47" : "#3b455b", borderRadius: 8 }}>{menu ? "ON" : "OFF"}</div>
      </div>
      {menu && (
        <anchored position={{ x: 24, y: 64 }} anchor="topLeft" deferred priority={1}>
          <div
            style={{
              padding: 14,
              backgroundColor: "#f5f8ff",
              color: "#172033",
              borderRadius: 8,
            }}
            onMouseDownOutside={() => setMenu(false)}
          >
            GPUI deferred overlay: clicks outside pass through.
          </div>
        </anchored>
      )}
      <SplitView direction="horizontal" defaultRatio={0.5} minSize={220} minSecondSize={220} style={{ flexGrow: 1, minHeight: 0 }}>
        <Browser label="Browser A" profileId={BROWSER_A_PROFILE} url={browserAUrl} reloadRequestId={String(reloadA)} clearDataRequestId={String(clearA)} onEvent={record} />
        <SplitView direction="vertical" defaultRatio={0.55} minSize={160} minSecondSize={160}>
          <Browser label="Browser B" profileId={BROWSER_B_PROFILE} visible={showSecond} url={page("Browser B")} onEvent={record} />
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", backgroundColor: "#131c31", borderRadius: 12, padding: 12, gap: 6 }}>
            <div>GPUI base pane</div>
            {events.map((event) => <text key={event} style={{ fontSize: 12, color: "#aebfe8" }}>{event}</text>)}
          </div>
        </SplitView>
      </SplitView>
    </div>
  )
}

render(<App />, { title: "GPUIX browser compositor proof", width: 1280, height: 820 })
