const appKey = (document.head.querySelector('meta[name="vapid-public-key"]') as HTMLMetaElement).content

const registrationPromise = navigator.serviceWorker.register('/static/js/service-worker.js', {
  scope: '/',
  // this is not supported in FireFox yet, as of v111
  // type: 'module'
})

function base64ToUint8Array (base64: string) {
  const bin = atob(base64.replaceAll('-', '+').replaceAll('_', '/'))
  const arr = new Uint8Array(bin.length)
  for (let i = 0; i < arr.length; i++) {
    arr[i] = bin.charCodeAt(i)
  }
  return arr
}

export async function subscribeToPushNotifications (optIn: string[]): Promise<SubscriptionState> {
  const registration = await registrationPromise
  if (!registration.pushManager) {
    throw new Error('Push notifications are not supported in this browser.')
  }
  
  const permission = await Notification.requestPermission()
  if (permission === 'denied') {
    throw new Error('Please check browser settings to allow notifications from this site.')
  }
  
  let subscription: PushSubscription
  try {
    subscription = await registration.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: base64ToUint8Array(appKey)
    })
  } catch (e) {
    throw new Error('Could not enable push notifications. Please check your browser settings.\n\n' + e.toString())
  }
  // console.log('subscribed to push', subscription)
  const body = {
    subscription: subscription.toJSON(),
    opt_in: optIn
  }  
  try {
    const resp = await fetch('/push/register', {
      method: 'POST',
      body: JSON.stringify(body),
      headers: {
        'Content-Type': 'application/json'
      },
      credentials: 'same-origin'
    })
    if (resp.ok) {
      return await resp.json() as SubscriptionState
    } else {
      const body = await resp.text()
      throw new Error(body)
    }
  } catch (e) {
    await subscription.unsubscribe()
    throw new Error('Failed to register for push notifications. Please try again later.\n\n' + e.toString())
  }
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

export interface SubscriptionState {
  opt_in: string[]
}

export async function pushSubscriptionState (): Promise<null | SubscriptionState> {
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

export async function sendTestPushNotification () {
  await fetch('/push/test', {
    method: 'POST',
    credentials: 'same-origin'
  })
}
