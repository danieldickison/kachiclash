(function () {
'use strict';

const root = document.getElementById('players');
const form = document.getElementById('player-list-form');

root.querySelectorAll('.select-broken-images').forEach(button => {
    button.addEventListener('click', () => {
        const service = button.innerText;
        form.querySelectorAll('tbody > tr').forEach(row => {
            const img = row.querySelector('.js-player-img');
            row.querySelector('.player-checkbox').checked = (
                row.dataset.service === service &&
                img.complete && 
                (
                    img.naturalHeight === 0 ||
                    img.src.endsWith('/static/img/oicho-silhouette.png')
                )
            );
        });
    });
});

root.querySelectorAll('.update-broken-images').forEach(button => {
    button.addEventListener('click', async () => {
        const data = {
            service: button.innerText.toLowerCase(),
            venue: bashoForm.elements.venue.value,
            start_date: bashoForm.elements.start_date.value,
            banzuke: parsedBanzuke,
        };
        const response = await fetch('/player/update_images', {
            method: 'POST',
            body: JSON.stringify(data),
            headers: new Headers({
                'Content-Type': 'application/json'
            }),
            credentials: 'same-origin',
        });
        if (response.ok) {
            const json = await response.json();
            console.log("response json:", json);
            //window.location = json.basho_url;
        } else {
            const text = await response.text();
            throw text;
        }
    })
});

})();
