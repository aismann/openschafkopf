enum EPlayerIndex { EPI0=0, EPI1, EPI2, EPI3, } // TODO "numeric" enum necessary?

enum SCard {
    E7, E8, E9, EZ, EU, EO, EK, EA,
    G7, G8, G9, GZ, GU, GO, GK, GA,
    H7, H8, H9, HZ, HU, HO, HK, HA,
    S7, S8, S9, SZ, SU, SO, SK, SA,
}

interface Cards {
    veccard : Array<SCard>,
}

class SSiteState {
    readonly vectplstrstr_caption_message_zugeben: Array<[string, string]>;
    readonly msg: string;
    readonly ostich_current: null | Array<null | String>;
    readonly ostich_prev: null | Array<null | String>; // TODO good idea to have optionals?
    readonly oepi_winner_prev: null | number; // TODO should be together with ostich_prev
    readonly oepi_animate_card: null | number; // TODO should be derived from ostich_current
    readonly mapepistr: Array<string>;
}

let str_player_name = prompt("Name:");
let ws = new WebSocket("ws://localhost:8080");
ws.onopen = function(event) {
    ws.send(JSON.stringify({"str_player_name": str_player_name}));
};
ws.onmessage = function(msg) {
    let any_parsed = JSON.parse(msg.data);
    console.log(any_parsed);
    if (Array.isArray(any_parsed.vectplstrstr_caption_message_zugeben)) {
        let div_hand = document.createElement("DIV");
        div_hand.id = "hand";
        for (let x of any_parsed.vectplstrstr_caption_message_zugeben) {
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
    if ("Ask" in any_parsed.msg) {
        console.log("ASK: " + any_parsed.msg["Ask"].vecstrgamephaseaction[0]);
    }
    if ("Ask" in any_parsed.msg && any_parsed.msg["Ask"].vecstrgamephaseaction) { // TODO is this the canonical emptiness check?
        console.log("ASK: " + any_parsed.msg["Ask"]);
        let div_askpanel_new = document.createElement("DIV");
        div_askpanel_new.id = "askpanel";
        let paragraph_title = document.createElement("p");
        paragraph_title.appendChild(document.createTextNode(any_parsed.msg.Ask.str_question));
        div_askpanel_new.appendChild(paragraph_title);
        let paragraph_btns = document.createElement("p");
        for (let x of any_parsed.msg["Ask"].vecstrgamephaseaction) {
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
        console.log(any_parsed.ostich_current);
        console.log("Most recent card: " + any_parsed.oepi_animate_card);
        let div_stich_new = document.createElement("DIV");
        div_stich_new.id = "stich";
        let i_epi = 0;
        for (i_epi = 0; i_epi<4; i_epi++) {
            let div_card = document.createElement("DIV");
            div_card.className = "card_stich card_stich_" + i_epi + " card";
            if (any_parsed.ostich_current[i_epi]) {
                div_card.className += " card_" + any_parsed.ostich_current[i_epi];
                if (any_parsed.oepi_animate_card==EPlayerIndex[i_epi]) {
                    div_card.style.animationDuration = "250ms";
                } else {
                    div_card.style.animationDuration = "0s";
                }
            }
            div_stich_new.appendChild(div_card);
        }
        let div_stich_old = document.getElementById("stich");
        div_stich_old.parentNode.replaceChild(div_stich_new, div_stich_old);
    }
    {
        console.log(any_parsed.ostich_prev);
        let div_stich_new = document.createElement("DIV");
        div_stich_new.id = "stich_old";
        let i_epi = 0;
        for (i_epi = 0; i_epi<4; i_epi++) {
            let div_card = document.createElement("DIV");
            div_card.className = "card_stich card_stich_" + i_epi + " card";
            if (any_parsed.ostich_prev[i_epi]) {
                div_card.className += " card_" + any_parsed.ostich_prev[i_epi];
            }
            if (
                any_parsed.ostich_current
                && !any_parsed.ostich_current[0]
                && !any_parsed.ostich_current[1]
                && !any_parsed.ostich_current[2]
                && !any_parsed.ostich_current[3]
            ) {
                div_stich_new.style.animationDuration = "250ms";
            } else {
                div_stich_new.style.animationDuration = "0s";
            }
            div_stich_new.appendChild(div_card);
        }
        let div_stich_old = document.getElementById("stich_old");
        div_stich_old.parentNode.replaceChild(div_stich_new, div_stich_old);
    }
    {
        console.log(any_parsed.oepi_winner_prev);
        if (any_parsed.oepi_winner_prev) {
            let div_stich_old = document.getElementById("stich_old");
            div_stich_old.className = "stich_old_" + any_parsed.oepi_winner_prev;
        }
    }
    {
        console.log(any_parsed.mapepistr);
        let i_epi = 0;
        for (i_epi = 0; i_epi<4; i_epi++) {
            let div_player = document.getElementById("playerpanel_player_" + i_epi);
            div_player.textContent = any_parsed.mapepistr[i_epi];
        }
    }
};
