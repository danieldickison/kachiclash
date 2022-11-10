const banzukeSection = document.getElementById('banzuke')
const pickForm = document.getElementById('banzuke-select-rikishi-form')

for (const button of banzukeSection.querySelectorAll('.toggle-pick-counts')) {
  button.addEventListener('click', event => {
    event.preventDefault()
    banzukeSection.classList.toggle('hide-pick-counts')
  })
}

document.querySelectorAll('.select-radio').forEach(radio => {
  radio.addEventListener('change', _event => {
    document.getElementsByName(radio.name).forEach(otherRadio => {
      otherRadio.closest('td').classList.toggle('is-player-pick', otherRadio === radio)
    })
    // savePicks();
  })
})

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
document.querySelectorAll('.change-picks-button').forEach(button => {
  button.addEventListener('click', event => {
    event.preventDefault()
    setSelectable(true)
  })
})

function setSelectable (selectable) {
  banzukeSection.classList.toggle('selectable', selectable)
  document.querySelectorAll('.select-radio').forEach(button => {
    button.disabled = !selectable
  })
}

async function savePicks (formData, url) {
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

document.querySelectorAll('.bestow-emperors-cup-button').forEach(button => {
  button.addEventListener('click', () => postCup(button, true))
})
document.querySelectorAll('.revoke-emperors-cup-button').forEach(button => {
  button.addEventListener('click', () => postCup(button, false))
})

function postCup (button, bestow) {
  const data = {
    player_id: parseInt(button.dataset.playerId)
  }
  const url = location.href + '/' + (bestow ? 'bestow' : 'revoke') + '_emperors_cup'
  return fetch(url, {
    method: 'POST',
    body: JSON.stringify(data),
    headers: new Headers({
      'Content-Type': 'application/json'
    }),
    credentials: 'same-origin'
  })
    .then(response => {
      if (response.ok) {
        alert("Emperor's Cup has been " + (bestow ? 'bestowed' : 'revoked'))
      } else {
        response.text().then(text => alert('error: ' + text))
      }
    })
}
