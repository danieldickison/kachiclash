import {
  pushPermissionState, pushSubscriptionState, sendTestPushNotification, subscribeToPushNotifications, SubscriptionState, unsubscribeFromPushNotifications
} from "./service-client.js"

const dialog = document.getElementById('notification-settings') as HTMLDialogElement
const form = dialog.querySelector('form') as HTMLFormElement
const buttons = form.elements['buttons'] as HTMLFieldSetElement
const toggleButton = buttons.elements['toggle'] as HTMLButtonElement
const testButton = buttons.elements['test'] as HTMLButtonElement
const types = form.elements['types'] as HTMLFieldSetElement
const typeCheckboxes = types.querySelectorAll('input[type="checkbox"]') as NodeListOf<HTMLInputElement>

let subscriptionState: SubscriptionState | null = null

export async function showNotificationSettings () {
  dialog.showModal()
  await refreshState()
}

function updateUi (busy: boolean) {
  buttons.disabled = busy
  toggleButton.innerText = busy ? 'notifications…' : subscriptionState ? 'disable notifications' : 'enable notifications'
  testButton.disabled = busy || !subscriptionState
  
  types.disabled = busy || !subscriptionState
  if (!busy) {
    for (const checkbox of typeCheckboxes) {
     checkbox.checked = subscriptionState && subscriptionState.opt_in.includes(checkbox.name)
    }
  }
}

async function refreshState () {
  updateUi(true)
  const permission = await pushPermissionState()
  subscriptionState = permission ? await pushSubscriptionState() : null
  updateUi(false)
}

async function submit () {
  updateUi(true)
  try {
    if (subscriptionState) {
      await unsubscribeFromPushNotifications()
      subscriptionState = null
    } else {
      subscriptionState = await subscribeToPushNotifications(getOptInTypes())
    }
  } catch (error) {
    alert(error.toString())
  } finally {
    updateUi(false)
  }
}

function getOptInTypes () {
  const types: string[] = []
  for (const checkbox of typeCheckboxes) {
    if (checkbox.checked) {
      types.push(checkbox.name)
    }
  }
  return types
}

toggleButton.addEventListener('click', async event => {
  event.preventDefault()
  await submit()
})
  
testButton.addEventListener('click', async event => {
  event.preventDefault()
  await sendTestPushNotification()
})

for (const checkbox of typeCheckboxes) {
  checkbox.addEventListener('change', submit)
}
