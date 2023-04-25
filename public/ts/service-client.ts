const appKey = (document.head.querySelector('meta[name="vapid-public-key"]') as HTMLMetaElement).content

const registrationPromise = navigator.serviceWorker.register('/static/js/service-worker.js', {
  scope: '/',
  // this is not supported in FireFox yet, as of v111
  // type: 'module'
})

function base64ToUint8Array(base64: string) {
  const bin = atob(base64.replaceAll('-', '+').replaceAll('_', '/'))
  const arr = new Uint8Array(bin.length)
  for (let i = 0; i < arr.length; i++) {
    arr[i] = bin.charCodeAt(i)
  }
  return arr
}

export async function subscribeToPushNotifications(): Promise<PushSubscription> {
  const registration = await registrationPromise
  if (!registration.pushManager) {
    throw new Error('Push notifications are not supported in this browser.')
  }

  const permission = await Notification.requestPermission()
  if (permission === 'denied') {
    throw new Error('Please check browser settings to allow notifications from this site.')
  }

  try {
    return await registration.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: base64ToUint8Array(appKey)
    })
  } catch (e) {
    throw new Error('Could not enable push notifications. Please check your browser settings.\n\n' + e.toString())
  }
}

export async function unsubscribeFromPushNotifications() {
  const registration = await registrationPromise
  const subscription = await registration.pushManager.getSubscription()
  return await subscription?.unsubscribe()
}

export type PushPermissionState = PermissionState | 'unavailable'

export async function pushPermissionState() {
  const registration = await registrationPromise
  if (!registration.pushManager) {
    return Promise.resolve('unavailable' as PushPermissionState)
  } else {
    return await registration.pushManager.permissionState({
      userVisibleOnly: true,
      applicationServerKey: appKey
    }) as PushPermissionState
  }
}

export interface SubscriptionState {
  opt_in: string[]
}

export async function pushSubscriptionState(): Promise<null | SubscriptionState> {
  const registration = await registrationPromise
  const subscription = await registration.pushManager.getSubscription()
  if (!subscription) {
    return null
  }

  const resp = await fetch('/push/check', {
    method: 'POST',
    body: JSON.stringify(subscription.toJSON()),
    headers: {
      'Content-Type': 'application/json'
    },
    credentials: 'same-origin'
  })

  if (resp.ok) {
    return await resp.json() as SubscriptionState

  } else if (resp.status === 404) {
    alert('Push notification registration has been lost. Please re-subscribe.')
    await subscription.unsubscribe()
    return null

  } else {
    const body = await resp.text()
    throw new Error(body)
  }
}