let ws = new WebSocket('ws://127.0.0.1:3012');

// see card.rs
let EFarbe = Object.freeze({
    "Eichel":0,
    "Gras":1,
    "Herz":2,
    "Schelln":3,
});
let ESchlag = Object.freeze({
    "Ass":0,
    "Zehn":1,
    "Koenig":2,
    "Ober":3,
    "Unter":4,
    "S9":5,
    "S8":6,
    "S7":7,
})

function addToDocInParagraph(content) {
    let paragraph = document.createElement("p");
    paragraph.appendChild(document.createTextNode(content));
    document.body.appendChild(paragraph);
}

function getProperty(publicinfo, prop) {
    if (prop in publicinfo) {
        addToDocInParagraph(prop + ": " + publicinfo[prop]);
    }
}

function displayHand(publicinfo) {
    // TODO something similar to assert("hand" in publicinfo)
    let veccard = publicinfo["hand"];
    console.log(veccard);
    for (let card of veccard) {
        let n_width = 336 / 8; // TODO divide by ESchlag.size
        let n_height = 232 / 4; // TODO divide by EFarbe size
        let div = document.createElement("div");
        div.style.margin = 0;
        div.style.padding = 0;
        div.style.width = n_width + "px";
        div.style.height = n_height + "px";
        div.style.display = 'inline-block';
        div.style.backgroundImage = 'url(https://www.sauspiel.de/images/redesign/cards/by/card-icons@2x.png)';
        console.log(card);
        div.style.backgroundPositionX = (-n_width * Object.freeze({
                "Ass":0,
                "Zehn":1,
                "Koenig":2,
                "Ober":3,
                "Unter":4,
                "S9":5,
                "S8":6,
                "S7":7,
            })[card[1]]) + 'px';
        div.style.backgroundPositionY = (-n_height * {
                "Eichel":0,
                "Gras":1,
                "Herz":2,
                "Schelln":3,
            }[card[0]]) + 'px';
        div.style.border = "none"; // TODO support solid for first card in stich
        document.body.appendChild(div);
    }
}

function unpack_gamestate(publicinfo) {
    // must correspond to VGamePhase
    if ('DealCards' in publicinfo) {
        return publicinfo['DealCards'];
    }
    if ('GamePreparations' in publicinfo) {
        return publicinfo['GamePreparations'];
    }
    if ('DetermineRules' in publicinfo) {
        return publicinfo['DetermineRules'];
    }
    if ('Game' in publicinfo) {
        return publicinfo['Game'];
    }
    if ('GameResult' in publicinfo) {
        return publicinfo['GameResult'];
    }
    console.log('Error: Unknown gamestate.');
}

ws.onmessage = function(msg) {
    console.log("Received: " + msg.data);
    console.log("type(msg): " + typeof(msg));
    console.log("type(msg.data): " + typeof(msg.data));
    let json = JSON.parse(msg.data); // TODO error handling
    console.log(json);
    // process available information
    let publicinfo = json[0];
    console.log(publicinfo);
    let gamestate = unpack_gamestate(publicinfo);
    console.log(gamestate);
    getProperty(gamestate, "hand");
    displayHand(gamestate);
    getProperty(gamestate, "doublings");
    getProperty(gamestate, "n_stock");
    getProperty(gamestate, "vecpairepistr_queued");
    getProperty(gamestate, "pairepistr_current_bid");
    getProperty(gamestate, "str_rules");
    getProperty(gamestate, "vecstoss");
    getProperty(gamestate, "ostossparams");
    getProperty(gamestate, "vecstich");
    getProperty(gamestate, "accountbalance");
    // process available commands
    let allowedactions = json[1];
    console.log(allowedactions);
    allowedactions.forEach(function(e) {
        let btn = document.createElement("BUTTON");
        btn.appendChild(document.createTextNode(e[0]));
        btn.onclick = function () {
            console.log(e);
            ws.send(JSON.stringify(e[1]));
        };
        document.body.appendChild(btn);
    });
    window.scrollTo(0, document.body.scrollHeight);
}
