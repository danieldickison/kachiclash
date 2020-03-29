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
                img.complete && 
                (
                    img.naturalHeight === 0 ||
                    img.src === '/static/img/oicho-silhouette.png'
                )
            );
        });
    });
});

})();
