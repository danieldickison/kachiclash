const appKey = (
  document.head.querySelector(
    'meta[name="vapid-public-key"]',
  ) as HTMLMetaElement
).content;

// Support for declarative web push notifications:
// https://webkit.org/blog/16535/meet-declarative-web-push/#how-to-use-declarative-web-push
declare global {
  interface Window {
    pushManager: PushManager | undefined;
  }
}

const pushManager = initPushManager();

async function initPushManager(): Promise<PushManager | undefined> {
  if ("pushManager" in window) {
    await unregisterServiceWorker();
    return window.pushManager;
  } else if ("serviceWorker" in navigator) {
    const serviceWorker = await navigator.serviceWorker?.register(
      "/static/js/service-worker.js",
      {
        scope: "/",
        // this is not supported in FireFox yet, as of v111
        // type: 'module'
      },
    );
    return serviceWorker.pushManager;
  } else {
    return undefined;
  }
}

async function unregisterServiceWorker() {
  const serviceWorker = await navigator.serviceWorker?.getRegistration();
  if (serviceWorker === undefined) {
    console.debug("No existing service worker");
    return;
  }
  if (await serviceWorker.unregister()) {
    console.info("Unregistered existing service worker", serviceWorker);
  } else {
    console.warn("Failed to unregister existing service worker", serviceWorker);
  }
}

function base64ToUint8Array(base64: string): Uint8Array {
  const bin = atob(base64.replaceAll("-", "+").replaceAll("_", "/"));
  const arr = new Uint8Array(bin.length);
  for (let i = 0; i < arr.length; i++) {
    arr[i] = bin.charCodeAt(i);
  }
  return arr;
}

export async function subscribeToPushNotifications(): Promise<PushSubscription> {
  const pm = await pushManager;
  if (pm === undefined || !("Notification" in window)) {
    throw new Error("Push notifications are not supported in this browser.");
  }

  const permission = await Notification.requestPermission();
  if (permission === "denied") {
    throw new Error(
      "Please check browser settings to allow notifications from this site.",
    );
  }

  try {
    return await pm.subscribe({
      userVisibleOnly: true,
      applicationServerKey: base64ToUint8Array(appKey),
    });
  } catch (e: any) {
    throw new Error(
      `Could not enable push notifications. Please check your browser settings.\n\n${
        e.toString() as string
      }`,
    );
  }
}

export type PushPermissionState = PermissionState | "unavailable";

export async function pushPermissionState(): Promise<PushPermissionState> {
  const pm = await pushManager;
  if (pm === undefined || !("Notification" in window)) {
    return "unavailable";
  } else {
    // It seems that in Safari, these three methods of getting the permission state are sometimes divergent, so we'll take all three and return 'granted' if any of them say so; otherwise use navigator.permissions as the source of truth. https://developer.apple.com/forums/thread/731412
    const pushPerm = await pm.permissionState({
      userVisibleOnly: true,
      applicationServerKey: base64ToUint8Array(appKey),
    });
    const notifPerm = Notification.permission;
    const queryPerm = (
      await navigator.permissions.query({ name: "notifications" })
    ).state;
    if (
      pushPerm === "granted" ||
      queryPerm === "granted" ||
      notifPerm === "granted"
    ) {
      return "granted";
    } else {
      return queryPerm;
    }
  }
}

export interface SubscriptionState {
  opt_in: string[];
}

export async function pushSubscriptionState(): Promise<null | SubscriptionState> {
  const pm = await pushManager;
  const subscription = await pm?.getSubscription();
  if (subscription === null || subscription === undefined) {
    return null;
  }

  const resp = await fetch("/push/check", {
    method: "POST",
    body: JSON.stringify(subscription.toJSON()),
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "same-origin",
  });

  if (resp.ok) {
    return (await resp.json()) as SubscriptionState;
  } else if (resp.status === 404) {
    alert("Push notification registration has been lost. Please re-subscribe.");
    await subscription.unsubscribe();
    return null;
  } else {
    const body = await resp.text();
    throw new Error(body);
  }
}
