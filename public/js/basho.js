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
    const form = document.getElementById('banzuke-select-rikishi-form');
    const data = new URLSearchParams(new FormData(form));
    const url = form.action;
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

const countDown = document.getElementById('basho-count-down');
if (countDown) {
    const startTimestamp = parseInt(countDown.dataset.startDate);
    const timeSpan = document.getElementById('basho-count-down-time');
    updateTimeRemaining();
    setInterval(updateTimeRemaining, 1000);

    function updateTimeRemaining() {
        const remaining = (startTimestamp - Date.now()) / 1000;
        const seconds = Math.floor(remaining % 60);
        const minutes = Math.floor(remaining / 60) % 60;
        const hours = Math.floor(remaining / 60 / 60) % 24;
        const days = Math.floor(remaining / 60 / 60 / 24);
        let str = "";

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

document.querySelectorAll('.bestow-emperors-cup-button').forEach(button => {
    button.addEventListener('click', () => postCup(button, true));
});
document.querySelectorAll('.revoke-emperors-cup-button').forEach(button => {
    button.addEventListener('click', () => postCup(button, false));
});

function postCup(button, bestow) {
    const data = {
        player_id: parseInt(button.dataset.playerId)
    };
    const url = location.href + '/' + (bestow ? 'bestow' : 'revoke') + '_emperors_cup';
    return fetch(url, {
        method: 'POST',
        body: JSON.stringify(data),
        headers: new Headers({
            'Content-Type': 'application/json'
        }),
    })
    .then(response => {
        if (response.ok) {
            alert("Emperor's Cup has been " + (bestow ? "bestowed" : "revoked"));
        } else {
            response.text().then(text => alert("error: " + text));
        }
    });
}
