const bashoForm = document.getElementById('make-basho-form') as HTMLFormElement

interface HTMLFormControlsCollection extends HTMLCollectionBase {
  // [item: string]: HTMLElement | RadioNodeList
  banzuke: HTMLInputElement
  venue: HTMLInputElement
  start_date: HTMLInputElement
}

let parsedBanzuke
bashoForm.elements.banzuke.addEventListener('input', bashoFormInput)

function bashoFormInput (event) {
  parsedBanzuke = parseBanzuke(bashoForm.elements.banzuke.value)
  const tbody = bashoForm.querySelector('.parsed-banzuke tbody')
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

function parseBanzuke (str) {
  const rikishi = []
  let match
  while ((match = BANZUKE_REGEX.exec(str))) {
    rikishi.push({
      rank: match[1],
      name: match[2],
      is_kyujyo: !!match[3]
    })
  }
  return rikishi
}

bashoForm.addEventListener('submit', event => {
  event.preventDefault()
  const data = {
    venue: bashoForm.elements.venue.value,
    start_date: bashoForm.elements.start_date.value,
    banzuke: parsedBanzuke
  }
  const url = location.href
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
        return response.json()
      } else {
        return response.text().then(msg => { throw msg })
      }
    })
    .then(json => {
      console.log('json:', json)
      window.location = json.basho_url
    })
    .catch(err => alert('error saving basho: ' + err))
})

bashoFormInput(null)
