import { useState } from "react"
import type { EventPayload } from "@regenrek/gpuix-native"
import { render } from "../../index.js"

function LiveAutomationHost() {
  const [value, setValue] = useState("")
  const [commandEvents, setCommandEvents] = useState<string[]>([])

  return (
    <div
      testId="live-automation-host"
      style={{
        width: "100%",
        height: "100%",
        backgroundColor: "#18181b",
        color: "#fafafa",
        padding: 24,
      }}
    >
      <input
        testId="live-automation-input"
        value={value}
        placeholder="Type here"
        style={{ width: 240, height: 32 }}
        onChange={(event: EventPayload) => setValue(event.value ?? "")}
        onKeyDown={(event: EventPayload) => {
          if (event.key === "c" && event.modifiers?.cmd) {
            setCommandEvents((events) => [...events, "down:cmd-c"])
          }
        }}
        onKeyUp={(event: EventPayload) => {
          if (event.key === "c" && event.modifiers?.cmd) {
            setCommandEvents((events) => [...events, "up:cmd-c"])
          }
        }}
      />
      <text>{`Value: ${value}`}</text>
      <text>{`Command events: ${commandEvents.join(",") || "none"}`}</text>
      <div
        testId="live-automation-scroll"
        style={{ width: 240, height: 48, overflowY: "scroll" }}
      >
        <div style={{ height: 240 }}>
          <text>Scrollable live content</text>
        </div>
      </div>
    </div>
  )
}

render(
  <LiveAutomationHost />,
  { title: "GPUIX automation live host", width: 320, height: 180 }
)
