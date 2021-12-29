fn dbg_print(x) {
    debug(x);
    print(x);
    x
}

fn inspect(params) {
    switch params.rules.to_string() {
        "HerzSolo" => {
            let epi = params.rules.playerindex();
            let hand = params.ahand[epi];
            dbg_print(hand);
            let n_trumpf = params.trumpf(epi);
            print(n_trumpf);
            return "Hallo";
        }
        _ => {
            print("Warning: Unknown rule type.");
            return
        }
    }
}
