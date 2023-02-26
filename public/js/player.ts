import {
  isSubscribedForPush,
  pushPermissionState, sendTestPushNotification, subscribeToPushNotifications,
  unsubscribeFromPushNotifications
} from "./service-client.js"

const profileSection = document.getElementById('profile')
const toggleEditMode = (event: Event) => {
  event.preventDefault()
  profileSection.classList.toggle('editing')
}
for (const edit of profileSection.querySelectorAll('.buttons > .edit')) {
  edit.addEventListener('click', toggleEditMode)
}
for (const cancel of profileSection.querySelectorAll(':scope > form .cancel')) {
  cancel.addEventListener('click', toggleEditMode)
}

async function initPushButtons () {
  const togglePushButtons = profileSection.querySelectorAll('.toggle-push') as NodeListOf<HTMLButtonElement>
  const testPushButtons = profileSection.querySelectorAll('.test-push') as NodeListOf<HTMLButtonElement>
  const permission = await pushPermissionState()
  let subscribed = await isSubscribedForPush()
  
  const updateState = (busy: boolean) => {
    const label = busy ? 'notifications…' : subscribed ? 'disable notifications' : 'enable notifications'
    for (const toggle of togglePushButtons) {
      toggle.disabled = busy
      toggle.innerText = label
    }
    for (const test of testPushButtons) {
      test.disabled = busy || !subscribed
    }
  }
  
  updateState(false)
  
  for (const toggle of togglePushButtons) {
    toggle.addEventListener('click', async event => {
      event.preventDefault()
      updateState(true)
      if (subscribed) {
        await unsubscribeFromPushNotifications()
        subscribed = false
      } else {
        subscribed = await subscribeToPushNotifications()
      }
      updateState(false)
    })
  }
  
  for (const button of testPushButtons) {
    button.addEventListener('click', async event => {
      event.preventDefault()
      await sendTestPushNotification()
    })
  }
}

// Note: *don't* await here, since we want other scripts to continue executing while we figuer out the push button state.
initPushButtons()
