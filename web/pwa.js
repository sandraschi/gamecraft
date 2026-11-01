// Registers the arcade service worker (see pwa.ts). Skipped in verify mode, so
// the gate always measures the live files, never a cache.
if ("serviceWorker" in navigator && !new URLSearchParams(location.search).has("verify")) {
  navigator.serviceWorker.register("/sw.js", { scope: "/" }).catch((err) => {
    console.warn("GameCraft: offline support unavailable:", err?.message ?? err);
  });
}
