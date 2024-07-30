const banzukeSection = document.getElementById('banzuke') as HTMLElement
const pickForm = document.getElementById('banzuke-select-rikishi-form') as HTMLFormElement
const heyaSelect = document.getElementById('heya-select') as HTMLSelectElement | null

for (const el of document.querySelectorAll('.select-radio')) {
  const radio = el as HTMLInputElement
  radio.addEventListener('change', _event => {
    for (const otherRadio of document.getElementsByName(radio.name)) {
      const label = pickForm.querySelector(`label.click-target[for="${otherRadio.id}"]`) as HTMLElement
      label.classList.toggle('is-player-pick', otherRadio === radio)
    }
    // savePicks();
  })
}

pickForm.addEventListener('submit', event => {
  event.preventDefault()
  const formData = new FormData(pickForm)
  const url = pickForm.action
  setSelectable(false)
  void (async function () {
    const success = await savePicks(formData, url)
    if (success) {
      location.reload()
    } else {
      setSelectable(true)
    }
  })()
})
for (const button of document.querySelectorAll('.change-picks-button')) {
  button.addEventListener('click', event => {
    event.preventDefault()
    setSelectable(true)
  })
}

function setSelectable (selectable: boolean): void {
  banzukeSection.classList.toggle('selectable', selectable)
  for (const el of document.querySelectorAll('.select-radio')) {
    const button = el as HTMLInputElement
    button.disabled = !selectable
  }
}

async function savePicks (formData: FormData, url: string): Promise<boolean> {
  const data = new URLSearchParams(formData as unknown as any) // https://github.com/microsoft/TypeScript-DOM-lib-generator/pull/880
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

heyaSelect?.addEventListener('change', () => heyaSelect?.form?.submit())

export default {}
