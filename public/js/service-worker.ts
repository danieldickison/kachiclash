// Hack to make TypeScript work in ServiceWorker env:
// https://github.com/Microsoft/TypeScript/issues/14877
// https://github.com/microsoft/TypeScript/issues/20595
declare const self: any
export default null

self.addEventListener("install", e => {
  e.waitUntil(self.skipWaiting())
})

self.addEventListener('push', e => {
  
})

