'use strict';

let bashoForm = document.getElementById('make-basho-form');

var parsedBanzuke;
bashoForm.elements.banzuke.addEventListener('input', (event) => {
    parsedBanzuke = parseBanzuke(bashoForm.elements.banzuke.value);
    let tbody = bashoForm.querySelector('.parsed-banzuke tbody');
    tbody.innerHTML = '';
    parsedBanzuke.forEach(rikishi => {
        let tr = document.createElement('tr');
        tbody.appendChild(tr);

        let rank = document.createElement('td');
        rank.innerText = rikishi.rank;
        tr.appendChild(rank);

        let name = document.createElement('td');
        name.innerText = rikishi.name;
        tr.appendChild(name);
    });
});

// Maches rank and name
let BANZUKE_REGEX = /^ *(\w{1,2}\d{1,3}[ew]) *(\w+)/gm

function parseBanzuke(str) {
    let rikishi = [];
    var match;
    while (match = BANZUKE_REGEX.exec(str)) {
        rikishi.push({
            rank: match[1],
            name: match[2],
        });
    }
    return rikishi;
}

bashoForm.addEventListener('submit', event => {
    event.preventDefault();
    let data = {
            venue: bashoForm.elements.venue.value,
            start_date: bashoForm.elements.start_date.value,
            banzuke: parsedBanzuke,
        };
    let url = bashoForm.action;
    return fetch(url, {
        method: 'POST',
        body: JSON.stringify(data),
    })
    .then(response => {
        if (response.ok) {
            return response.json();
        } else {
            throw "error saving basho";
        }
    })
    .then(json => {
        console.log("json:", json);
        window.location = json.basho_url;
    })
    .catch(err => alert(err));
});
