#![cfg_attr(feature = "cargo-clippy", allow(clippy::blocks_in_if_conditions, clippy::just_underscores_and_digits))]
#![deny(bare_trait_objects)]

#[macro_use]
mod util;
mod ai;
mod game;
mod game_analysis;
mod player;
mod primitives;
mod rules;
mod skui;
mod subcommands;

use crate::primitives::*;
use crate::util::*;

fn main() -> Result<(), Error> {
    openschafkopf_logging::init_logging()?;
    let clap_arg = |str_long, str_default| {
        clap::Arg::with_name(str_long)
            .long(str_long)
            .default_value(str_default)
    };
    // TODO clean up command line arguments and possibly avoid repetitions
    let clapmatches = clap::App::new("schafkopf")
        .subcommand(clap::SubCommand::with_name("cli")
            .about("Simulate players to play against")
            .arg(clap_arg("ruleset", "rulesets/default.toml"))
            .arg(clap_arg("ai", "cheating"))
            .arg(clap_arg("numgames", "4"))
        )
        .subcommand(clap::SubCommand::with_name("rank-rules")
            .about("Estimate strength of own hand")
            .arg(clap_arg("ruleset", "rulesets/default.toml"))
            .arg(clap_arg("ai", "cheating"))
            .arg(clap_arg("hand", ""))
            .arg(clap_arg("position", "0"))
        )
        .subcommand({
            let single_arg = |str_name, str_long| {
                clap::Arg::with_name(str_name)
                    .long(str_long)
                    .required(true)
                    .takes_value(true)
            };
            clap::SubCommand::with_name("suggest-card")
                .about("Suggest a card to play given the game so far")
                .arg(single_arg("rules", "rules"))
                .arg(single_arg("hand", "hand"))
                .arg(single_arg("cards_on_table", "cards-on-table"))
                .arg(clap::Arg::with_name("branching").long("branching").takes_value(true))
                .arg(clap::Arg::with_name("simulate_hands").long("simulate-hands").takes_value(true))
                .arg(clap::Arg::with_name("verbose").long("verbose").short("v"))
                .arg(clap::Arg::with_name("prune").long("prune").takes_value(true))
                .arg(clap::Arg::with_name("constrain_hands").long("constrain-hands").takes_value(true))
                .arg(clap::Arg::with_name("batch").long("batch").takes_value(true).required(true))
        })
        .subcommand(clap::SubCommand::with_name("analyze")
            .about("Analyze played games and spot suboptimal decisions")
            .arg(clap::Arg::with_name("sauspiel-files")
                 .required(true)
                 .takes_value(true)
                 .multiple(true)
            )
        )
        .subcommand(clap::SubCommand::with_name("websocket")
            .arg(clap_arg("ruleset", "rulesets/default.toml"))
        )
        .get_matches();
    if let Some(clapmatches_websocket)=clapmatches.subcommand_matches("websocket") {
        return subcommands::websocket::run(clapmatches_websocket);
    }
    if let Some(clapmatches_analyze)=clapmatches.subcommand_matches("analyze") {
        return subcommands::analyze::analyze(clapmatches_analyze);
    }
    if let Some(clapmatches_rank_rules)=clapmatches.subcommand_matches("rank-rules") {
        return subcommands::rank_rules::rank_rules(clapmatches_rank_rules);
    }
    if let Some(clapmatches_suggest_card)=clapmatches.subcommand_matches("suggest-card") {
        return subcommands::suggest_card::suggest_card(clapmatches_suggest_card);
    }
    if let Some(clapmatches_cli)=clapmatches.subcommand_matches("cli") {
        return subcommands::cli::game_loop_cli(clapmatches_cli);
    }
    Ok(())
}


