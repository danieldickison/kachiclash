'use strict';

document.querySelectorAll('.select-radio').forEach(radio => {
    radio.addEventListener('change', event => {
        document.getElementsByName(radio.name).forEach(otherRadio => {
            otherRadio.closest('td').classList.toggle('is-player-pick', otherRadio === radio);
        });
        savePicks();
    });
});

function savePicks() {
    let form = document.getElementById('banzuke-select-rikishi-form');
    let data = new URLSearchParams(new FormData(form));
    let url = form.action;
    return fetch(url, {
        method: 'POST',
        //credentials: 'same-origin', // include, *same-origin, omit
        body: data,
    })
    .then(response => {
        if (!response.ok) {
            response.text().then(text => alert("error saving your pick: " + text));
        }
    });
}

let countDown = document.getElementById('basho-count-down');
if (countDown) {
    let startTimestamp = parseInt(countDown.dataset.startDate);
    let timeSpan = document.getElementById('basho-count-down-time');
    updateTimeRemaining();
    setInterval(updateTimeRemaining, 1000);

    function updateTimeRemaining() {
        let remaining = (startTimestamp - Date.now()) / 1000;
        let seconds = Math.floor(remaining % 60);
        let minutes = Math.floor(remaining / 60) % 60;
        let hours = Math.floor(remaining / 60 / 60) % 24;
        let days = Math.floor(remaining / 60 / 60 / 24);
        var str = "";

        if (days > 1) str += days + " days ";
        else if (days > 0) str += "1 day ";

        if (hours > 1) str += hours + " hours ";
        else if (hours === 1) str += "1 hour ";
        else if (days > 0) str += "0 hours ";

        if (minutes > 1) str += minutes + " minutes ";
        else if (minutes === 1) str += "1 minute ";
        else if (hours > 0) str += "0 minutes ";

        if (seconds > 1) str += seconds + " seconds ";
        else if (seconds === 1) str += "1 second ";
        else if (minutes > 0) str += "0 seconds ";

        timeSpan.innerText = str.trim();
    }
}
