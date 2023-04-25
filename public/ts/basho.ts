const banzukeSection = document.getElementById('banzuke')
const pickForm = document.getElementById('banzuke-select-rikishi-form') as HTMLFormElement

for (const radio of document.querySelectorAll('.select-radio') as NodeListOf<HTMLInputElement>) {
  radio.addEventListener('change', _event => {
    for (const otherRadio of document.getElementsByName(radio.name)) {
      const label = pickForm.querySelector(`label.click-target[for="${otherRadio.id}"]`)
      label.classList.toggle('is-player-pick', otherRadio === radio)
    }
    // savePicks();
  })
}

pickForm.addEventListener('submit', async (event) => {
  event.preventDefault()
  const formData = new FormData(pickForm)
  const url = pickForm.action
  setSelectable(false)
  const success = await savePicks(formData, url)
  if (success) {
    location.reload()
  } else {
    setSelectable(true)
  }
})
for (const button of document.querySelectorAll('.change-picks-button')) {
  button.addEventListener('click', event => {
    event.preventDefault()
    setSelectable(true)
  })
}

function setSelectable(selectable) {
  banzukeSection.classList.toggle('selectable', selectable)
  for (const button of document.querySelectorAll('.select-radio') as NodeListOf<HTMLInputElement>) {
    button.disabled = !selectable
  }
}

async function savePicks(formData, url) {
  const data = new URLSearchParams(formData)
  const response = await fetch(url, {
    method: 'POST',
    body: data,
    credentials: 'same-origin'
  })
  if (response.ok) {
    alert('Your picks have been saved!')
    return true
  } else {
    const text = await response.text()
    alert('Error saving your picks: ' + text)
    return false
  }
}

export default {}