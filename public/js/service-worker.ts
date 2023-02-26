// Hack to make TypeScript work in ServiceWorker env:
// https://github.com/Microsoft/TypeScript/issues/14877
// https://github.com/microsoft/TypeScript/issues/20595
//
// Alas, FireFox doesn't yet support module workers as of v111 so we can't do this yet.
// declare const self: any
// export default null

// Workaround by doing iife:
(function (self: any) {
self.addEventListener("install", (e: any) => {
  e.waitUntil(self.skipWaiting())
})

self.addEventListener('push', (e: any) => {
  const { msg, data } = e.data.json() as PushPayload
  e.waitUntil(
    self.registration.showNotification(
      "Kachi Clash (title)",
      {
        body: msg
      }
    )
  )
})

interface PushPayload {
  msg: String,
  data: EntriesOpen | BashoStartCountdown | DayResult
}

// Keep in sync with data/push.rs
type EntriesOpen = {
    basho_id: number,
    start_date: number,
}
type BashoStartCountdown = {
    basho_id: number,
    start_date: number,
}
type DayResult = {
    basho_id: number,
    name: string,
    day: number,
    rikishi: [RikishiDayResult],
    rank: number,
    leaders: [string],
    leader_score: number,
}
type RikishiDayResult = {
  name: string,
  won: null | boolean,
  against: null | string,
  wins: number,
  losses: number,
  absence: number,
}
})(self)
