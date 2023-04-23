import { sendTestNotification } from "./push.js"
import {
  pushPermissionState, pushSubscriptionState, subscribeToPushNotifications, SubscriptionState
} from "./service-client.js"

const form = document.getElementById('settings-form') as HTMLFormElement
const messages = form.querySelector('.messages') as HTMLElement
const saveButton = form.querySelector('.save-button') as HTMLButtonElement
const nameField = form.elements['name'] as HTMLInputElement
const testNotificationButton = form.elements['test-notification'] as HTMLButtonElement
const notifications = form.elements['notifications'] as HTMLFieldSetElement
const shareMenuButton = notifications.querySelector('.share-menu') as HTMLAnchorElement
const typeCheckboxes = notifications.querySelectorAll('input[type="checkbox"]') as NodeListOf<HTMLInputElement>

let subscriptionState: SubscriptionState | null = null
let edited = false

notifications.classList.toggle('ios', /iPhone|iPad/.test(navigator.userAgent))

form.addEventListener('submit', async event => {
  event.preventDefault()
  await save()
})
form.addEventListener('input', _event => {
  showMessage(false, '')
  edited = true
  refreshBusyState(false)
})

testNotificationButton.addEventListener('click', async event => {
  event.preventDefault()
  await sendTestNotification()
})

shareMenuButton.addEventListener('click', async event => {
  event.preventDefault()
  await navigator.share()
})

async function refreshState() {
  refreshBusyState(true)
  const permission = await pushPermissionState()
  notifications.dataset.permissionState = permission
  subscriptionState = permission === 'granted' ? await pushSubscriptionState() : null
  for (const checkbox of typeCheckboxes) {
    checkbox.checked = subscriptionState && subscriptionState.opt_in.includes(checkbox.value)
  }
  refreshBusyState(false)
}

function refreshBusyState(busy: boolean) {
  form.classList.toggle('busy', busy)
  saveButton.disabled = busy || !edited
  notifications.disabled = busy
  nameField.disabled = busy
  testNotificationButton.disabled = busy || !subscriptionState
}

async function save() {
  refreshBusyState(true)
  showMessage(false, '')
  try {
    let pushSubscription = null
    const optIn = getOptInTypes()
    if (optIn.length > 0) {
      pushSubscription = await subscribeToPushNotifications()
    }

    const body = {
      name: nameField.value,
      push_subscription: pushSubscription?.toJSON(),
      notification_opt_in: optIn
    }
    const resp = await fetch('/settings', {
      method: 'POST',
      body: JSON.stringify(body),
      headers: {
        'Content-Type': 'application/json'
      },
      credentials: 'same-origin'
    })
    if (resp.ok) {
      showMessage(false, 'Settings have been saved.')
    } else {
      const body = await resp.text()
      throw new Error(body)
    }
  } catch (error) {
    showMessage(true, error.toString())
  } finally {
    edited = false
    await refreshState()
    refreshBusyState(false)
  }
}

function showMessage(isError: boolean, message: string) {
  messages.style.display = message ? 'block' : 'none'
  messages.classList.toggle('error', isError)
  messages.innerText = message
  messages.scrollIntoView()
}

function getOptInTypes() {
  const types: string[] = []
  for (const checkbox of typeCheckboxes) {
    if (checkbox.checked) {
      types.push(checkbox.value)
    }
  }
  return types
}

refreshState()
