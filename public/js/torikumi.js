(function () {
'use strict';

const torikumiForm = document.getElementById('torikumi-form');

let parsedTorikumi;
torikumiForm.elements.torikumi.addEventListener('input', torikumiFormInput);

function torikumiFormInput() {
    parsedTorikumi = parseTorikumi(torikumiForm.elements.torikumi.value);
    const tbody = torikumiForm.querySelector('.parsed-torikumi tbody');
    tbody.innerHTML = '';
    parsedTorikumi.forEach(torikumi => {
        const tr = document.createElement('tr');
        tbody.appendChild(tr);

        const winner = document.createElement('td');
        winner.innerText = torikumi.winner;
        tr.appendChild(winner);

        const loser = document.createElement('td');
        loser.innerText = torikumi.loser;
        tr.appendChild(loser);
    });
}

// Maches rank, name, record, kimarite, rank, name, record
const TORIKUMI_REGEX = /^ *\w{1,2}\d{1,3}[ew] +(\w+) +\(\d+.\d+\) +\w+ *\w{1,2}\d{1,3}[ew] +(\w+) +\(\d+.\d+\) *$/gm

function parseTorikumi(str) {
    console.log("parsing torikumi");
    const torikumi = [];
    let match;
    while (match = TORIKUMI_REGEX.exec(str)) {
        torikumi.push({
            winner: match[1],
            loser: match[2],
        });
    }
    return torikumi;
}

torikumiForm.addEventListener('submit', event => {
    event.preventDefault();
    const data = {
            torikumi: parsedTorikumi,
        };
    const postURL = location.href;
    const bashoURL = postURL.replace(/\/day\/.*$/i, '');
    return fetch(postURL, {
        method: 'POST',
        body: JSON.stringify(data),
        headers: new Headers({
            'Content-Type': 'application/json'
        }),
    })
    .then(response => {
        if (response.ok) {
            window.location = bashoURL;
        } else {
            return response.text().then(msg => {throw msg});
        }
    })
    .catch(err => alert("error updating torikumi: " + err));
});

torikumiFormInput();
})();
