import { alertSendStats } from './push.js'

const bashoForm = document.getElementById('make-basho-form') as HTMLFormElement

declare global {
  interface HTMLFormControlsCollection extends HTMLCollectionBase {
  // [item: string]: HTMLElement | RadioNodeList
    banzuke: HTMLInputElement
    venue: HTMLInputElement
    start_date: HTMLInputElement
    notify_kyujyo: HTMLInputElement
  }
}

let parsedBanzuke: Rikishi[]
bashoForm.elements.banzuke.addEventListener('input', bashoFormInput)

function bashoFormInput (): void {
  parsedBanzuke = parseBanzuke(bashoForm.elements.banzuke.value)
  const tbody = bashoForm.querySelector('.parsed-banzuke tbody') as HTMLTableSectionElement
  tbody.innerHTML = ''
  parsedBanzuke.forEach(rikishi => {
    const tr = document.createElement('tr')
    tbody.appendChild(tr)

    const rank = document.createElement('td')
    rank.innerText = rikishi.rank
    tr.appendChild(rank)

    const name = document.createElement('td')
    name.innerText = rikishi.name
    tr.appendChild(name)

    const kyujyo = document.createElement('td')
    kyujyo.innerText = rikishi.is_kyujyo ? '㊡' : ''
    tr.appendChild(kyujyo)
  })
}

// Maches rank and name
const BANZUKE_REGEX = /^ *(\w{1,2}\d{1,3}[ew]) *(\w+).*?( x)?$/gm

interface Rikishi {
  rank: string
  name: string
  is_kyujyo: boolean
}

function parseBanzuke (str: string): Rikishi[] {
  const rikishi: Rikishi[] = []
  let match: any[] | null
  while (((match = BANZUKE_REGEX.exec(str)) !== null)) {
    rikishi.push({
      rank: match[1] as string,
      name: match[2] as string,
      is_kyujyo: match[3] !== undefined
    })
  }
  return rikishi
}

bashoForm.addEventListener('submit', event => {
  event.preventDefault()
  const data = {
    venue: bashoForm.elements.venue.value,
    start_date: bashoForm.elements.start_date.value,
    banzuke: parsedBanzuke,
    notify_kyujyo: bashoForm.elements.notify_kyujyo.checked
  }
  const url = location.href
  fetch(url, {
    method: 'POST',
    body: JSON.stringify(data),
    headers: new Headers({
      'Content-Type': 'application/json'
    }),
    credentials: 'same-origin'
  })
    .then(async response => {
      if (response.ok) {
        return await response.json()
      } else {
        return await response.text().then(msg => { throw new Error(msg) })
      }
    })
    .then(json => {
      console.log('json:', json)
      alertSendStats(json.notification_stats)
      window.location = json.basho_url
    })
    .catch((err: Error) => { alert(`error saving basho: ${err.toString()}`) })
})

bashoFormInput()
