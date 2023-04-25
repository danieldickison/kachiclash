// Hack to make TypeScript work in ServiceWorker env:
// https://github.com/Microsoft/TypeScript/issues/14877
// https://github.com/microsoft/TypeScript/issues/20595
//
// Alas, FireFox doesn't yet support module workers as of v111 so we can't do this yet.
// declare const self: any
// export default null
//
// Workaround by doing iife:
(function(self: any) {
  const VERSION = 1

  self.addEventListener('install', (e: any) => {
    console.debug(`Installing version ${VERSION}`)
    // kick out old workers immediately
    e.waitUntil(self.skipWaiting())
  })

  self.addEventListener('activate', (e: any) => {
    console.debug(`Activating version ${VERSION}`)
    // claim any pages that loaded without a worker so we can focus them on notification click
    e.waitUntil(self.clients.cliam())
  })

  self.addEventListener('push', (e: any) => {
    const { title, body, ...data } = e.data.json() // as Payload // needs to be esmodule to import
    console.debug('Received push notification with data', data)
    e.waitUntil(
      self.registration.showNotification(title, { body, data })
    )
  })

  self.addEventListener('notificationclick', async (e: any) => {
    const notification = e.notification as Notification
    notification.close()
    e.waitUntil(openOrFocusClient(notification.data.url))
  })

  async function openOrFocusClient(url: string) {
    let client = (await self.clients.matchAll())[0]
    // todo: maybe match client url with deets from notification

    console.debug(`opening ${url} in ${client ? 'existing' : 'new'} window`)
    client ||= await self.clients.openWindow(url)
    client.focus()
  }
})(self)
