import {
  isSubscribedForPush,
  pushPermissionState,
  subscribeToPushNotifications,
  unsubscribeFromPushNotifications
} from "./service-client.js"

const profileSection = document.getElementById('profile')
const toggleEditMode = (event) => {
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
  const permission = await pushPermissionState()
  let subscribed = await isSubscribedForPush()
  
  const updateState = (disabled: boolean) => {
    const label = subscribed ? 'disable notifications' : 'enable notifications'
    for (const toggle of togglePushButtons) {
      toggle.disabled = disabled
      toggle.innerText = label
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
}

// Note: *don't* await here, since we want other scripts to continue executing while we figuer out the push button state.
initPushButtons()
