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
