enum EPlayerIndex { EPI0=0, EPI1, EPI2, EPI3, } // TODO can we simplify enum interop with serde?

enum SCard {
    E7, E8, E9, EZ, EU, EO, EK, EA,
    G7, G8, G9, GZ, GU, GO, GK, GA,
    H7, H8, H9, HZ, HU, HO, HK, HA,
    S7, S8, S9, SZ, SU, SO, SK, SA,
}

interface Cards {
    veccard : Array<SCard>,
}

class Ask {
    str_question: string;
    vecstrgamephaseaction: Array<[string, any]>;
}
class Ask_ {
    Ask: Ask;
}

function isAsk(msg: string | Ask_) : msg is Ask_ {
    return (msg as Ask_).Ask !== undefined;
}
function getAsk(msg: string | Ask_) : Ask | null {
    if (isAsk(msg)) {
        return msg.Ask;
    } else {
        return null;
    }
}

class SSiteState {
    readonly vectplstrstr_caption_message_zugeben: Array<[string, string]>;
    readonly msg: string | Ask_;
    readonly ostich_current: null | Array<null | string>;
    readonly ostich_prev: null | Array<null | string>; // TODO good idea to have optionals?
    readonly oepi_winner_prev: null | EPlayerIndex; // TODO should be together with ostich_prev
    readonly oepi_animate_card: null | EPlayerIndex; // TODO should be derived from ostich_current
    readonly mapepistr: Array<string>;
    readonly otplepistr_rules: null | [EPlayerIndex, string]
    readonly oepi_timeout: null | EPlayerIndex;
}

function assert(b) {
    if (!b) {
        throw {};
    }
}

let str_player_name = prompt("Name:");
let ws = new WebSocket("ws://localhost:8080");
ws.onopen = function(event) {
    ws.send(JSON.stringify({"PlayerLogin": {"str_player_name": str_player_name}}));
};
ws.onmessage = function(msg) {
    let sitestate = JSON.parse(msg.data) as SSiteState; // assume that server sends valid SSiteState // TODO? assert/check
    console.log(sitestate);
    {
        let div_hand = document.getElementById("hand");
        let vecdiv_card = div_hand.getElementsByClassName("card");
        if (vecdiv_card.length < sitestate.vectplstrstr_caption_message_zugeben.length) {
            for (let i = vecdiv_card.length; i<sitestate.vectplstrstr_caption_message_zugeben.length; i++) {
                let div_card = document.createElement("DIV");
                div_card.className = "card card_hand";
                div_hand.appendChild(div_card);
            }
        } else if (vecdiv_card.length > sitestate.vectplstrstr_caption_message_zugeben.length) {
            for (let i = sitestate.vectplstrstr_caption_message_zugeben.length; i<vecdiv_card.length; i++) {
                div_hand.children[0].remove();
            }
        }
        //assert_eq(vecdiv_card.length > sitestate.vectplstrstr_caption_message_zugeben.length);
        for (let i=0; i<vecdiv_card.length; i++) {
            let tplcardstr = sitestate.vectplstrstr_caption_message_zugeben[i];
            let vecstr_class = ["card", "card_hand", "card_" + tplcardstr[0]];
            let div_card = div_hand.children[i];
            assert(div_card.classList.contains(vecstr_class[0]));
            assert(div_card.classList.contains(vecstr_class[1]));
            if (!div_card.classList.contains(vecstr_class[2])) {
                div_card.className = vecstr_class.join(" ");
            }
            (<HTMLElement>div_card).onclick = function () {
                // TODO if (!player is active) { check } else
                console.log(tplcardstr[1]);
                ws.send(JSON.stringify({"GamePhaseAction": tplcardstr[1]}));
            };
        }
    }
    let div_askpanel = document.getElementById("askpanel");
    let oask = getAsk(sitestate.msg);
    if (oask) {
        console.log("ASK: " + oask.vecstrgamephaseaction[0]);
    }
    if (oask && oask.vecstrgamephaseaction) { // TODO is this the canonical emptiness check?
        console.log("ASK: " + oask);
        let div_askpanel_new = document.createElement("DIV");
        div_askpanel_new.id = "askpanel";
        let paragraph_title = document.createElement("p");
        paragraph_title.appendChild(document.createTextNode(oask.str_question));
        div_askpanel_new.appendChild(paragraph_title);
        let paragraph_btns = document.createElement("p");
        for (let x of oask.vecstrgamephaseaction) {
            console.log(x);
            let btn = document.createElement("BUTTON");
            btn.appendChild(document.createTextNode(JSON.stringify(x[0])));
            btn.onclick = function () {
                console.log(x[1]);
                ws.send(JSON.stringify({"GamePhaseAction": x[1]}));
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
        console.log(sitestate.ostich_current);
        console.log("Most recent card: " + sitestate.oepi_animate_card);
        let div_stich_new = document.createElement("DIV");
        div_stich_new.id = "stich";
        for (let i_epi = 0; i_epi<4; i_epi++) {
            let div_card = document.createElement("DIV");
            div_card.className = "card_stich card_stich_" + i_epi + " card";
            if (sitestate.ostich_current[i_epi]) {
                div_card.className += " card_" + sitestate.ostich_current[i_epi];
                if (sitestate.oepi_animate_card==i_epi) {
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
        console.log(sitestate.ostich_prev);
        let div_stich_new = document.createElement("DIV");
        div_stich_new.id = "stich_old";
        for (let i_epi = 0; i_epi<4; i_epi++) {
            let div_card = document.createElement("DIV");
            div_card.className = "card_stich card_stich_" + i_epi + " card";
            if (sitestate.ostich_prev[i_epi]) {
                div_card.className += " card_" + sitestate.ostich_prev[i_epi];
            }
            if (
                sitestate.ostich_current
                && !sitestate.ostich_current[0]
                && !sitestate.ostich_current[1]
                && !sitestate.ostich_current[2]
                && !sitestate.ostich_current[3]
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
        console.log(sitestate.oepi_winner_prev);
        if (null!==sitestate.oepi_winner_prev) {
            let div_stich_old = document.getElementById("stich_old");
            div_stich_old.className = "stich_old_" + sitestate.oepi_winner_prev;
        }
    }
    {
        console.log(sitestate.mapepistr);
        console.log(sitestate.oepi_timeout);
        for (let i_epi = 0; i_epi<4; i_epi++) {
            let div_player = document.getElementById("playerpanel_player_" + i_epi);
            div_player.textContent = sitestate.mapepistr[i_epi];
            if (sitestate.oepi_timeout===i_epi) {
                div_player.className = "playerpanel_active";
            } else {
                div_player.className = "";
            }
        }
    }
    {
        console.log(sitestate.otplepistr_rules);
        if (sitestate.otplepistr_rules) {
            let div_player = document.getElementById("playerpanel_player_" + sitestate.otplepistr_rules[0]);
            div_player.textContent += ": " + sitestate.otplepistr_rules[1];
        }
    }
};
