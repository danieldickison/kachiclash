import { adminTrigger } from "./push.js"

document.querySelectorAll('.bestow-emperors-cup-button').forEach(button => {
  button.addEventListener('click', () => postCup(button, true))
})
document.querySelectorAll('.revoke-emperors-cup-button').forEach(button => {
  button.addEventListener('click', () => postCup(button, false))
})

async function postCup(button, bestow) {
  const data = {
    player_id: parseInt(button.dataset.playerId)
  }
  const url = location.href + '/' + (bestow ? 'bestow' : 'revoke') + '_emperors_cup'
  const response = await fetch(url, {
    method: 'POST',
    body: JSON.stringify(data),
    headers: new Headers({
      'Content-Type': 'application/json'
    }),
    credentials: 'same-origin'
  })
  if (response.ok) {
    alert("Emperor's Cup has been " + (bestow ? 'bestowed' : 'revoked'))
  } else {
    response.text().then(text => alert('error: ' + text))
  }
}

const bashoId = (document.querySelector('meta[name="basho_id"]') as HTMLMetaElement).content

document.querySelector('.trigger-announcement').addEventListener('click', event => {
  event.preventDefault()
  const msg = prompt('Message:')
  adminTrigger({ Announcement: msg })
})
document.querySelector('.trigger-entries-open').addEventListener('click', event => {
  event.preventDefault()
  adminTrigger({ EntriesOpen: bashoId })
})
document.querySelector('.trigger-countdown').addEventListener('click', event => {
  event.preventDefault()
  adminTrigger({ BashoStartCountdown: bashoId })
})

export default {}
