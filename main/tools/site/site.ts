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
    document.body.textContent = document.body.textContent + "<br/>" + msg.data;
};
ws.send("Initial message");
