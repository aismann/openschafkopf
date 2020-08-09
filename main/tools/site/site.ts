function greeter(person: string) {
    return "hallo: " + person;    
}

let user = "Nutzer Name";

document.body.textContent = greeter(user);

enum EPlayerIndex { EPI0, EPI1, EPI2, EPI3, }

enum SCard {
    E7, E8, E9, EZ, EU, EO, EK, EA,
    G7, G8, G9, GZ, GU, GO, GK, GA,
    H7, H8, H9, HZ, HU, HO, HK, HA,
    S7, S8, S9, SZ, SU, SO, SK, SA,
}

interface Cards {
    veccard : Array<SCard>,
}

let ws = new WebSocket("ws://localhost:8080");
ws.onmessage = function(msg) {
    let any_parsed = JSON.parse(msg.data);
    console.log(any_parsed);
    // any_parsed[0]: EPlayerIndex
    // any_parsed[1]: vectplstrstr_caption_message_zugeben
    // any_parsed[2]: VMessage
    // any_parsed[3]: stich (relative to EPlayerIndex, so that client does not have to shift)
    // any_parsed[4]: previous stich (relative to EPlayerIndex, so that client does not have to shift)
    // any_parsed[5]: (optional) winner index of previous stich // TODO should be part of previous stich
    if (Array.isArray(any_parsed[1])) {
        let div_hand = document.createElement("DIV");
        div_hand.id = "hand";
        for (let x of any_parsed[1]) {
            console.log(x);
            let div_card = document.createElement("DIV");
            div_card.className = "card card_hand card_" + x[0];
            div_card.onclick = function () {
                console.log(x[1]);
                ws.send(JSON.stringify(x[1]));
            };
            div_hand.appendChild(div_card);
        }
        let div_hand_old = document.getElementById("hand");
        console.log(div_hand_old);
        console.log(div_hand_old.parentNode);
        div_hand_old.parentNode.replaceChild(div_hand, div_hand_old);
    }
    let div_askpanel = document.getElementById("askpanel");
    if ("Ask" in any_parsed[2] && any_parsed[2]["Ask"]) { // TODO is this the canonical emptiness check?
        let paragraph_btns = document.createElement("p");
        let div_askpanel_new = document.createElement("DIV");
        div_askpanel_new.id = "askpanel";
        for (let x of any_parsed[2]["Ask"]) {
            console.log(x);
            let btn = document.createElement("BUTTON");
            btn.appendChild(document.createTextNode(JSON.stringify(x[0])));
            btn.onclick = function () {
                console.log(x[1]);
                ws.send(JSON.stringify(x[1]));
            };
            paragraph_btns.appendChild(btn);
            div_askpanel_new.appendChild(paragraph_btns);
            //window.scrollTo(0, document.body.scrollHeight);
        }
        div_askpanel.parentNode.replaceChild(div_askpanel_new, div_askpanel);
    } else {
        div_askpanel.hidden = true;
    }
    {
        console.log(any_parsed[3]);
        let div_stich_new = document.createElement("DIV");
        div_stich_new.id = "stich";
        let i_epi = 0;
        for (i_epi = 0; i_epi<4; i_epi++) {
            let div_card = document.createElement("DIV");
            div_card.className = "card_stich card_stich_" + i_epi + " card";
            if (any_parsed[3][i_epi]) {
                div_card.className += " card_" + any_parsed[3][i_epi];
            }
            div_stich_new.appendChild(div_card);
        }
        let div_stich_old = document.getElementById("stich");
        div_stich_old.parentNode.replaceChild(div_stich_new, div_stich_old);
    }
    {
        console.log(any_parsed[4]);
        let div_stich_new = document.createElement("DIV");
        div_stich_new.id = "stich_old";
        let i_epi = 0;
        for (i_epi = 0; i_epi<4; i_epi++) {
            let div_card = document.createElement("DIV");
            div_card.className = "card_stich card_stich_" + i_epi + " card";
            if (any_parsed[4][i_epi]) {
                div_card.className += " card_" + any_parsed[4][i_epi];
            }
            div_stich_new.appendChild(div_card);
        }
        let div_stich_old = document.getElementById("stich_old");
        div_stich_old.parentNode.replaceChild(div_stich_new, div_stich_old);
    }
    {
        console.log(any_parsed[5]);
        if (any_parsed[5]) {
            let div_stich_old = document.getElementById("stich_old");
            switch(any_parsed[5]){
                case "EPI0": {
                    div_stich_old.style.left = "40%";
                    div_stich_old.style.top = "70%";
                    break;
                }
                case "EPI1": {
                    div_stich_old.style.left = "5%";
                    div_stich_old.style.top = "40%";
                    break;
                }
                case "EPI2": {
                    div_stich_old.style.left = "40%";
                    div_stich_old.style.top = "5%";
                    break;
                }
                case "EPI3": {
                    div_stich_old.style.left = "75%";
                    div_stich_old.style.top = "40%";
                    break;
                }
            }
        }
    }
};
