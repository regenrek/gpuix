import type {
  BrowserDataClearedEvent,
  BrowserDownloadDestinationSelectedEvent,
  BrowserLoadingEvent,
  BrowserNavigationEvent,
  BrowserSurfaceProps,
} from "./index.js"

const loading: BrowserLoadingEvent = {
  elementId: 1,
  eventType: "browserLoading",
  browserUrl: "https://example.com",
  browserIsLoading: true,
  browserCanGoBack: false,
  browserCanGoForward: false,
  browserProfileId: "profile",
}

const navigation: BrowserNavigationEvent = {
  elementId: 1,
  eventType: "browserNavigation",
  browserUrl: "https://example.com",
  browserCanGoBack: false,
  browserCanGoForward: false,
  browserProfileId: "profile",
}

const dataCleared: BrowserDataClearedEvent = {
  elementId: 1,
  eventType: "browserDataCleared",
  browserProfileId: "profile",
  browserRequestId: "clear-1",
}

const destinationSelected: BrowserDownloadDestinationSelectedEvent = {
  elementId: 1,
  eventType: "browserDownloadDestinationSelected",
  browserProfileId: "profile",
  browserRequestId: "download-1",
  browserDownloadId: "profile:1",
  browserSelectedFileUrl: "file:///selected.txt",
}

const browserSurfaceProps: BrowserSurfaceProps = {
  profileId: "profile",
  onBrowserLoading: (event) => {
      const typed: BrowserLoadingEvent = event
      void typed.browserIsLoading
  },
  onBrowserNavigation: (event) => {
      const typed: BrowserNavigationEvent = event
      void typed.browserCanGoBack
  },
  onBrowserDataCleared: (event) => {
      const typed: BrowserDataClearedEvent = event
      void typed.browserRequestId
  },
  onBrowserDownloadDestinationSelected: (event) => {
      const typed: BrowserDownloadDestinationSelectedEvent = event
      void typed.browserSelectedFileUrl
  },
}

export const browserPublicContract = <browser-surface {...browserSurfaceProps} />

void loading
void navigation
void dataCleared
void destinationSelected
