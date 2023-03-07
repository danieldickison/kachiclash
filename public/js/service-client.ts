const appKey = (document.head.querySelector('meta[name="vapid-public-key"]') as HTMLMetaElement).content

const registrationPromise = navigator.serviceWorker.register('/static/js/service-worker.js', {
  scope: '/',
  // this is not supported in FireFox yet, as of v111
  // type: 'module'
})

function base64ToUint8Array (base64) {
  const bin = atob(base64.replaceAll('-', '+').replaceAll('_', '/'))
  const arr = new Uint8Array(bin.length)
  for (let i = 0; i < arr.length; i++) {
    arr[i] = bin.charCodeAt(i)
  }
  return arr
}

export async function subscribeToPushNotifications () {
  const registration = await registrationPromise
  if (!registration.pushManager) {
    alert('Push notifications are not supported in this browser.')
    return false
  }
  
  const permission = await Notification.requestPermission()
  if (permission === 'denied') {
    alert('Please check system settings to allow browser-based notifications.')
    return false
  }
  
  let subscription: PushSubscription
  try {
    subscription = await registration.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: base64ToUint8Array(appKey)
    })
  } catch (e) {
    alert('Could not enable push notifications. Please check your browser settings.')
    return false
  }
  // console.log('subscribed to push', subscription)
  
  try {
    const resp = await fetch('/push/register', {
      method: 'POST',
      body: JSON.stringify(subscription.toJSON()),
      headers: {
        'Content-Type': 'application/json'
      },
      credentials: 'same-origin'
    })
    if (!resp.ok) {
      const body = await resp.text()
      throw new Error(body)
    }
  } catch (e) {
    await subscription.unsubscribe()
    alert('Failed to register for push notifications. Please try again later.\n\n' + e.toString())
    return false
  }
  
  return true
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

export async function sendTestPushNotification () {
  await fetch('/push/test', {
    method: 'POST',
    credentials: 'same-origin'
  })
}
