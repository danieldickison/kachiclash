// Init basho start count down clock
for (const timeSpan of document.querySelectorAll('.js-basho-count-down') as NodeListOf<HTMLElement>) {
  const startTimestamp = parseInt(timeSpan.dataset.startDate)
  const updateTimeRemaining = function () {
    const remaining = (startTimestamp - Date.now()) / 1000
    const seconds = Math.floor(remaining % 60)
    const minutes = Math.floor(remaining / 60) % 60
    const hours = Math.floor(remaining / 60 / 60) % 24
    const days = Math.floor(remaining / 60 / 60 / 24)
    let str = ''

    if (days > 1) str += days + ' days '
    else if (days > 0) str += '1 day '

    if (hours > 1) str += hours + ' hours '
    else if (hours === 1) str += '1 hour '
    else if (days > 0) str += '0 hours '

    if (minutes > 1) str += minutes + ' minutes '
    else if (minutes === 1) str += '1 minute '
    else if (hours > 0) str += '0 minutes '

    if (seconds > 1) str += seconds + ' seconds '
    else if (seconds === 1) str += '1 second '
    else if (minutes > 0) str += '0 seconds '

    timeSpan.innerText = str.trim()
  }

  updateTimeRemaining()
  setInterval(updateTimeRemaining, 1000)
}

// Show local time of basho start times
const DATETIME_FORMAT = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'long',
  day: 'numeric',
  hour: 'numeric',
  minute: 'numeric',
  timeZoneName: 'short'
})
for (const el of document.querySelectorAll('.js-local-datetime') as NodeListOf<HTMLElement>) {
  const timestamp = parseInt(el.dataset.timestamp)
  if (timestamp && !isNaN(timestamp)) {
    const date = new Date(timestamp)
    el.innerText = DATETIME_FORMAT.format(date)
  }
}

// Show standard placeholder for broken player avatar images
for (const img of document.querySelectorAll('img.js-player-img') as NodeListOf<HTMLImageElement>) {
  img.addEventListener('error', () => { img.src = '/static/img/oicho-silhouette.png' })
}

// User menu
const playerMenu = document.querySelector('#g-header .player-menu') as HTMLElement
const menuHeader = playerMenu.querySelector('.g-player-listing')
if (menuHeader instanceof HTMLAnchorElement) {
  const bodyClickHandler = (event: Event) => {
    const target = event.target
    if (target instanceof Element && !target.matches('.player-menu *')) {
      event.preventDefault()
      playerMenu.classList.remove('open')
      document.body.removeEventListener('click', bodyClickHandler, { capture: true })
    }
  }
  menuHeader.addEventListener('click', event => {
    event.preventDefault()
    playerMenu.classList.toggle('open')
    document.body.addEventListener('click', bodyClickHandler, { capture: true })
  })
}
