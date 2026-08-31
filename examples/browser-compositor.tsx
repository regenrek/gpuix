/**
 * macOS-only proof for GPUI Base / WKWebView / GPUI Overlay composition.
 * It intentionally uses local data URLs so the proof has no network dependency.
 */
import React, { createElement, useState } from "react"
import { render, SplitView, type BrowserActionDecision, type BrowserActionRequestedEvent, type BrowserNavigationIntent } from "@regenrek/gpuix-react"

const BROWSER_A_PROFILE = "1bf766d4-9632-4292-b2d4-6d7058cd78af"
const BROWSER_B_PROFILE = "a4eb96bc-1f22-471a-b9f8-bc7c2f7633a2"

const page = (label: string) =>
  `data:text/html;charset=utf-8,${encodeURIComponent(`<!doctype html><html><head><meta charset="utf-8"><title>${label}</title><style>html,body{background:#f8fbff;color:#182235}body{font:16px -apple-system;padding:24px}input,a{display:block;margin-top:16px;padding:10px;width:80%;background:#fff;color:#182235;border:1px solid #6e7d96}</style></head><body><h1>${label}</h1><p>Native WKWebView inside GPUIX.</p><input value="select and copy me"><a href="data:text/plain,gpuix-download-proof" download="gpuix-browser-compositor-proof.txt">Download proof</a></body></html>`)}`

type BrowserProps = {
  label: string
  profileId: string
  navigationIntent?: BrowserNavigationIntent
  actionDecision?: BrowserActionDecision
  clearDataRequestId?: string
  onEvent: (message: string) => void
  onActionRequested?: (event: BrowserActionRequestedEvent) => void
}

function Browser({ label, profileId, navigationIntent, actionDecision, clearDataRequestId, onEvent, onActionRequested }: BrowserProps) {
  return createElement("browser-surface", {
    profileId,
    testId: label === "Browser A" ? "browser-a" : "browser-b",
    navigationIntent,
    actionDecision,
    clearDataRequestId,
    style: { width: "100%", height: "100%", borderRadius: 12 },
    onBrowserNavigation: (event: { browserUrl: string; browserCanGoBack: boolean; browserCanGoForward: boolean }) =>
      onEvent(`${label}: navigated (${event.browserCanGoBack ? "back" : "start"}/${event.browserCanGoForward ? "forward" : "end"}) ${event.browserUrl}`),
    onBrowserLoading: (event: { browserIsLoading: boolean; browserUrl: string }) =>
      onEvent(`${label}: ${event.browserIsLoading ? "loading" : "loaded"} ${event.browserUrl}`),
    onBrowserActionRequested: (event: BrowserActionRequestedEvent) => {
      onEvent(`${label}: ${event.browserActionKind} ${event.browserRequestId}`)
      onActionRequested?.(event)
    },
    onBrowserDataCleared: (event: { browserProfileId: string; browserRequestId: string }) =>
      onEvent(`${label}: cleared ${event.browserProfileId} (${event.browserRequestId})`),
  })
}

function App() {
  const [menu, setMenu] = useState(false)
  const [showSecond, setShowSecond] = useState(true)
  const [browserAIntent, setBrowserAIntent] = useState<BrowserNavigationIntent>()
  const [browserADecision, setBrowserADecision] = useState<BrowserActionDecision>()
  const [browserBIntent] = useState<BrowserNavigationIntent>({ requestId: "browser-b-local", kind: "navigate", url: page("Browser B") })
  const [browserBDecision, setBrowserBDecision] = useState<BrowserActionDecision>()
  const [clearA, setClearA] = useState(0)
  const [events, setEvents] = useState<string[]>([])
  const record = (event: string) => setEvents((current) => [event, ...current].slice(0, 4))
  const decide = (setDecision: (decision: BrowserActionDecision) => void) => (event: BrowserActionRequestedEvent) => {
    if (event.browserActionKind === "downloadDestination") {
      setDecision({
        requestId: event.browserRequestId,
        decision: "save",
        destinationUrl: "file:///tmp/gpuix-browser-compositor-proof.txt",
      })
    } else if (
      event.browserShouldPerformDownload === true ||
      (event.browserActionKind === "navigationResponse" && event.browserCanShowMimeType === false)
    ) {
      setDecision({ requestId: event.browserRequestId, decision: "download" })
    } else if (event.browserUrl?.includes("blocked")) {
      setDecision({ requestId: event.browserRequestId, decision: "cancel" })
    } else {
      setDecision({ requestId: event.browserRequestId, decision: "allow" })
    }
  }
  return (
    <div style={{ width: "100%", height: "100%", display: "flex", flexDirection: "column", backgroundColor: "#0b1020", color: "#e8eefc", padding: 12, gap: 8 }}>
      <div style={{ display: "flex", gap: 8, height: 38 }}>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setMenu(!menu)}>Toggle GPUI menu</div>
        <div testId="browser-b-toggle" style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setShowSecond((show) => !show)}>{showSecond ? "Remove Browser B" : "Recreate Browser B"}</div>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setBrowserAIntent({ requestId: `allow-${Date.now()}`, kind: "navigate", url: page("Browser A") })}>Allow A navigation</div>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setBrowserAIntent({ requestId: `cancel-${Date.now()}`, kind: "navigate", url: "https://blocked.invalid" })}>Cancel A navigation</div>
        <div style={{ padding: 9, backgroundColor: "#263b70", borderRadius: 8, cursor: "pointer" }} onClick={() => setBrowserAIntent({ requestId: `reload-${Date.now()}`, kind: "reload" })}>Reload A</div>
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
        <Browser label="Browser A" profileId={BROWSER_A_PROFILE} navigationIntent={browserAIntent} actionDecision={browserADecision} clearDataRequestId={String(clearA)} onEvent={record} onActionRequested={decide(setBrowserADecision)} />
        <SplitView direction="vertical" defaultRatio={0.55} minSize={160} minSecondSize={160}>
          {showSecond ? (
            <Browser label="Browser B" profileId={BROWSER_B_PROFILE} navigationIntent={browserBIntent} actionDecision={browserBDecision} onEvent={record} onActionRequested={decide(setBrowserBDecision)} />
          ) : (
            <div testId="browser-b-removed" style={{ display: "flex", alignItems: "center", justifyContent: "center", backgroundColor: "#131c31", borderRadius: 12 }}>
              Browser B removed
            </div>
          )}
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
