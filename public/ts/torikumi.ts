const torikumiForm = document.getElementById('torikumi-form') as HTMLFormElement

// eslint-disable-next-line @typescript-eslint/no-unused-vars
interface HTMLFormControlsCollection extends HTMLCollectionBase {
  // [item: string]: HTMLElement | RadioNodeList
  torikumi: HTMLInputElement
  notify: HTMLInputElement
}

let parsedTorikumi: Torikumi[]
torikumiForm.elements.torikumi.addEventListener('input', torikumiFormInput)

function torikumiFormInput (): void {
  parsedTorikumi = parseTorikumi(torikumiForm.elements.torikumi.value)
  const tbody = torikumiForm.querySelector('.parsed-torikumi tbody') as HTMLTableSectionElement
  tbody.innerHTML = ''
  parsedTorikumi.forEach(torikumi => {
    const tr = document.createElement('tr')
    tbody.appendChild(tr)

    const winner = document.createElement('td')
    winner.innerText = torikumi.winner
    tr.appendChild(winner)

    const loser = document.createElement('td')
    loser.innerText = torikumi.loser
    tr.appendChild(loser)
  })
}

// Matches rank, name, record, kimarite, rank, name, record
const TORIKUMI_REGEX = /^ *[a-z]{1,2}\d{1,3}[ew] +([a-z]+) +\(\d+(?:-\d+){1,2}\) +[a-z]+ *[a-z]{1,2}\d{1,3}[ew] +([a-z]+) +\(\d+(?:-\d+){1,2}\) *$/gim

interface Torikumi {
  winner: string
  loser: string
}

function parseTorikumi (str: string): Torikumi[] {
  console.log('parsing torikumi')
  const torikumi: Torikumi[] = []
  let match: RegExpExecArray | null
  while (((match = TORIKUMI_REGEX.exec(str)) !== null)) {
    torikumi.push({
      winner: match[1],
      loser: match[2]
    })
  }
  return torikumi
}

torikumiForm.addEventListener('submit', event => {
  event.preventDefault()
  const data = {
    torikumi: parsedTorikumi,
    notify: torikumiForm.elements.notify.checked
  }
  const postURL = location.href
  const bashoURL = postURL.replace(/\/day\/.*$/i, '')
  fetch(postURL, {
    method: 'POST',
    body: JSON.stringify(data),
    headers: new Headers({
      'Content-Type': 'application/json'
    }),
    credentials: 'same-origin'
  })
    .then(async response => {
      if (response.ok) {
        window.location.href = bashoURL
      } else {
        return await response.text().then(msg => { throw new Error(msg) })
      }
    })
    .catch((err: Error) => { alert('error updating torikumi: ' + err.toString()) })
})

torikumiFormInput()
