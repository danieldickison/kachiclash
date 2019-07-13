'use strict';

document.querySelectorAll('.select-radio').forEach(radio => {
    radio.addEventListener('change', event => {
        document.getElementsByName(radio.name).forEach(otherRadio => {
            otherRadio.closest('label').classList.toggle('is-player-pick', otherRadio === radio);
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
