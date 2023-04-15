import {
  pushPermissionState, pushSubscriptionState, sendTestPushNotification, subscribeToPushNotifications, SubscriptionState
} from "./service-client.js"

const form = document.getElementById('settings-form') as HTMLFormElement
const messages = form.querySelector('.messages') as HTMLElement
const saveButton = form.querySelector('.save-button') as HTMLButtonElement
const nameField = form.elements['name'] as HTMLInputElement
const testNotificationButton = form.elements['test-notification'] as HTMLButtonElement
const notificationTypes = form.elements['notifications'] as HTMLFieldSetElement
const typeCheckboxes = notificationTypes.querySelectorAll('input[type="checkbox"]') as NodeListOf<HTMLInputElement>

let subscriptionState: SubscriptionState | null = null

form.addEventListener('submit', async event => {
  event.preventDefault()
  await save()
})

testNotificationButton.addEventListener('click', async event => {
  event.preventDefault()
  await sendTestPushNotification()
})

async function refreshState() {
  refreshBusyState(true)
  const permission = await pushPermissionState()
  subscriptionState = permission === 'granted' ? await pushSubscriptionState() : null
  for (const checkbox of typeCheckboxes) {
    checkbox.checked = subscriptionState && subscriptionState.opt_in.includes(checkbox.value)
  }
  refreshBusyState(false)
}

function refreshBusyState(busy: boolean) {
  form.classList.toggle('busy', busy)
  saveButton.disabled = busy
  notificationTypes.disabled = busy
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
    await refreshState()
    refreshBusyState(false)
  }
}

function showMessage(isError: boolean, message: string) {
  messages.style.display = message ? 'block' : 'none'
  messages.classList.toggle('error', isError)
  messages.innerText = message
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
