fn dbg_print(x) {
    debug(x);
    print(x);
}

fn inspect(ahand, rules) {
    switch rules.to_string() {
        "HerzSolo" => {
            let epi = rules.playerindex().unwrap();
            let hand = ahand[epi];
            dbg_print(hand);
            count(hand, rules, 3)
        }
        _ => {
            print("Unknown rule type.");
            return
        }
    }
}
