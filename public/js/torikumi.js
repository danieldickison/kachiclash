(function () {
'use strict';

let torikumiForm = document.getElementById('torikumi-form');

var parsedTorikumi;
torikumiForm.elements.torikumi.addEventListener('input', (event) => {
    parsedTorikumi = parseTorikumi(torikumiForm.elements.torikumi.value);
    let tbody = torikumiForm.querySelector('.parsed-torikumi tbody');
    tbody.innerHTML = '';
    parsedTorikumi.forEach(torikumi => {
        let tr = document.createElement('tr');
        tbody.appendChild(tr);

        let winner = document.createElement('td');
        winner.innerText = torikumi.winner;
        tr.appendChild(winner);

        let loser = document.createElement('td');
        loser.innerText = torikumi.loser;
        tr.appendChild(loser);
    });
});

// Maches rank, name, record, kimarite, rank, name, record
let TORIKUMI_REGEX = /^ *\w{1,2}\d{1,3}[ew] +(\w+) +\(\d+.\d+\) +\w+ +\w{1,2}\d{1,3}[ew] +(\w+) +\(\d+.\d+\) *$/gm

function parseTorikumi(str) {
    console.log("parsing torikumi");
    let torikumi = [];
    var match;
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
    let data = {
            torikumi: parsedTorikumi,
        };
    let url = window.location.href;
    return fetch(url, {
        method: 'POST',
        body: JSON.stringify(data),
        headers: new Headers({
            'Content-Type': 'application/json'
        }),
    })
    .then(response => {
        if (response.ok) {
            window.location.reload();
        } else {
            return response.text().then(msg => {throw msg});
        }
    })
    .catch(err => alert("error updating torikumi: " + err));
});
})();
