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
    // any_parsed[1]: VMessage
    if ("Ask" in any_parsed[1]) {
        let paragraph_btns = document.createElement("p");
        for (let x of any_parsed[1]["Ask"]) {
            console.log(x);
            let btn = document.createElement("BUTTON");
            btn.appendChild(document.createTextNode(JSON.stringify(x)));
            btn.onclick = function () {
                console.log(x);
                ws.send(JSON.stringify(x));
            };
            paragraph_btns.appendChild(btn);
            document.body.appendChild(paragraph_btns);
            window.scrollTo(0, document.body.scrollHeight);
        }
    }
};
