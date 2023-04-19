// Hack to make TypeScript work in ServiceWorker env:
// https://github.com/Microsoft/TypeScript/issues/14877
// https://github.com/microsoft/TypeScript/issues/20595
//
// Alas, FireFox doesn't yet support module workers as of v111 so we can't do this yet.
// declare const self: any
// export default null

// Workaround by doing iife:
(function(self: any) {
  self.addEventListener("install", (e: any) => {
    e.waitUntil(self.skipWaiting())
  })

  self.addEventListener('push', (e: any) => {
    const { title, body, ...data } = e.data.json() // as Payload // needs to be esmodule to import
    console.debug('Received push notification with data', data)
    e.waitUntil(
      self.registration.showNotification(title, { body })
    )
  })
})(self)
