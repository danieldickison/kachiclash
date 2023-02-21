// TODO: fetch this from server, or look it up in a <meta> tag
const appKey = 'BJjhuoxyYxOSuhas5a4963ghNYYlJzAneDwWpPGhrQehZNUMS8qbYhOyvxmOL0gDzyVoPTmw8o59wT87aPyXUnQ='

const registrationPromise = navigator.serviceWorker.register('/static/js/service-worker.js', {
  scope: '/',
  type: 'module'
})

export async function subscribeToPushNotifications () {
  const registration = await registrationPromise
  let subscription: PushSubscription
  try {
    subscription = await registration.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: appKey
    })
  } catch (e) {
    alert('Could not enable push notifications. Please check your browser settings.')
    return false
  }
  console.log('subscribed to push', subscription)
  await fetch('/push_register', {
    method: 'POST',
    body: JSON.stringify(subscription.toJSON()),
    headers: {
      'Content-Type': 'application/json'
    },
    credentials: 'same-origin'
  })
}

export async function unsubscribeFromPushNotifications () {
  const registration = await registrationPromise
  const subscription = await registration.pushManager.getSubscription()
  return await subscription?.unsubscribe()
}

export async function pushPermissionState () {
  const registration = await registrationPromise
  return await registration.pushManager.permissionState({
    userVisibleOnly: true,
    applicationServerKey: appKey
  })
}

export async function isSubscribedForPush () {
  const registration = await registrationPromise
  const subscription = await registration.pushManager.getSubscription()
  return !!subscription
}
