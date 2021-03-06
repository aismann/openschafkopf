use crate::game::{stoss_and_doublings, SGame, SStichSequence};
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;
use rand::{self, Rng};
use std::{cmp::Ordering, fmt, fs, io::Write};

pub trait TForEachSnapshot {
    type Output;
    fn final_output(&self, slcstich: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output;
    fn pruned_output(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<Self::Output>;
    fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
        &self,
        epi_card: EPlayerIndex,
        ittplcardoutput: ItTplCardOutput,
    ) -> Self::Output;
}

pub trait TSnapshotVisualizer<Output> {
    fn begin_snapshot(&mut self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>);
    fn end_snapshot(&mut self, output: &Output);
}


pub fn visualizer_factory<'rules>(path: std::path::PathBuf, rules: &'rules dyn TRules, epi: EPlayerIndex) -> impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, SCard) -> SForEachSnapshotHTMLVisualizer<'rules> {
    unwrap!(std::fs::create_dir_all(&path));
    unwrap!(crate::game_analysis::generate_html_auxiliary_files(&path));
    move |i_ahand, ahand, card| {
        let path_abs = path.join(
            &std::path::Path::new(&format!("{}", chrono::Local::now().format("%Y%m%d%H%M%S")))
                .join(format!("{}_{}_{}.html",
                    i_ahand,
                    ahand.iter()
                        .map(|hand| hand.cards().iter().join(""))
                        .join("_"),
                    card
                )),
        );
        unwrap!(std::fs::create_dir_all(unwrap!(path_abs.parent())));
        SForEachSnapshotHTMLVisualizer::new(
            unwrap!(std::fs::File::create(path_abs)),
            rules,
            epi
        )
    }
}

pub struct SForEachSnapshotHTMLVisualizer<'rules> {
    file_output: fs::File,
    rules: &'rules dyn TRules,
    epi: EPlayerIndex,
}
impl<'rules> SForEachSnapshotHTMLVisualizer<'rules> {
    fn new(file_output: fs::File, rules: &'rules dyn TRules, epi: EPlayerIndex) -> Self {
        let mut foreachsnapshothtmlvisualizer = SForEachSnapshotHTMLVisualizer{file_output, rules, epi};
        foreachsnapshothtmlvisualizer.write_all(
            b"<link rel=\"stylesheet\" type=\"text/css\" href=\"../css.css\">
            <style>
            input + label + ul {
                display: none;
            }
            input:checked + label + ul {
                display: block;
            }
            </style>"
        );
        foreachsnapshothtmlvisualizer
    }

    fn write_all(&mut self, buf: &[u8]) {
        if let Err(err) = self.file_output.write_all(buf) {
            error!("Error writing file: {}", err);
        }
    }
}

pub fn output_card(card: SCard, b_border: bool) -> String {
    format!(r#"<div class="card-image {}{}"></div>"#,
        card,
        if b_border {" border"} else {""},
    )
}

pub fn player_table<T: fmt::Display>(epi_self: EPlayerIndex, fn_per_player: impl Fn(EPlayerIndex)->Option<T>) -> String {
    let fn_per_player_internal = move |epi: EPlayerIndex| {
        fn_per_player(epi.wrapping_add(epi_self.to_usize()))
            .map_or("".to_string(), |t| t.to_string())
    };
    format!(
        "<table class=\"player-table\">
          <tr><td colspan=\"2\"><br>{}<br></td></tr>
          <tr><td>{}</td><td>{}</td></tr>
          <tr><td colspan=\"2\">{}</td></tr>
        </table>\n",
        fn_per_player_internal(EPlayerIndex::EPI2),
        fn_per_player_internal(EPlayerIndex::EPI1),
        fn_per_player_internal(EPlayerIndex::EPI3),
        fn_per_player_internal(EPlayerIndex::EPI0),
    )
}

impl TSnapshotVisualizer<SMinMax> for SForEachSnapshotHTMLVisualizer<'_> {
    fn begin_snapshot(&mut self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) {
        let str_item_id = format!("{}{}",
            stichseq.count_played_cards(),
            rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).join(""), // we simply assume no collisions here TODO uuid
        );
        self.write_all(format!("<li><<input type=\"checkbox\" id=\"{}\" />>\n", str_item_id).as_bytes());
        self.write_all(format!("<label for=\"{}\">{} direct successors<table><tr>\n",
            str_item_id,
            "TODO", // slccard_allowed.len(),
        ).as_bytes());
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(stichseq, ahand));
        for stich in stichseq.visible_stichs() {
            self.write_all(b"<td>\n");
            self.write_all(player_table(self.epi, |epi| stich.get(epi).map(|card| output_card(*card, epi==stich.first_playerindex()))).as_bytes());
            self.write_all(b"</td>\n");
        }
        let str_table_hands = format!(
            "<td>{}</td>\n",
            player_table(self.epi, |epi| {
                let mut veccard = ahand[epi].cards().clone();
                self.rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                Some(veccard.into_iter()
                    .map(|card| output_card(card, /*b_border*/false))
                    .join(""))
            }),
        );
        self.write_all(str_table_hands.as_bytes());
        self.write_all(b"</tr></table></label>\n");
        self.write_all(b"<ul>\n");
    }

    fn end_snapshot(&mut self, minmax: &SMinMax) {
        self.write_all(b"</ul>\n");
        self.write_all(b"</li>\n");
        self.write_all(player_table(self.epi, |epi| {
            Some(format!("{}/{}/{}/{}",
                minmax.t_min[epi],
                minmax.t_selfish_min[epi],
                minmax.t_selfish_max[epi],
                minmax.t_max[epi],
            ))
        }).as_bytes());
    }
}

pub struct SNoVisualization;
impl<Output> TSnapshotVisualizer<Output> for SNoVisualization {
    fn begin_snapshot(&mut self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>) {}
    fn end_snapshot(&mut self, _output: &Output) {}
}

pub fn explore_snapshots<ForEachSnapshot>(
    ahand: &mut EnumMap<EPlayerIndex, SHand>,
    rules: &dyn TRules,
    stichseq: &mut SStichSequence,
    func_filter_allowed_cards: &impl Fn(&SStichSequence, &mut SHandVector),
    foreachsnapshot: &ForEachSnapshot,
    snapshotvisualizer: &mut impl TSnapshotVisualizer<ForEachSnapshot::Output>,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
{
    explore_snapshots_internal(
        ahand,
        rules,
        &mut SRuleStateCache::new(
            stichseq,
            ahand,
            |stich| rules.winner_index(stich),
        ),
        stichseq,
        func_filter_allowed_cards,
        foreachsnapshot,
        snapshotvisualizer,
    )
}

fn explore_snapshots_internal<ForEachSnapshot>(
    ahand: &mut EnumMap<EPlayerIndex, SHand>,
    rules: &dyn TRules,
    rulestatecache: &mut SRuleStateCache,
    stichseq: &mut SStichSequence,
    func_filter_allowed_cards: &impl Fn(&SStichSequence, &mut SHandVector),
    foreachsnapshot: &ForEachSnapshot,
    snapshotvisualizer: &mut impl TSnapshotVisualizer<ForEachSnapshot::Output>,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
{
    snapshotvisualizer.begin_snapshot(stichseq, ahand);
    let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
    let output = if debug_verify_eq!(
        ahand[epi_current].cards().len() <= 1,
        ahand.iter().all(|hand| hand.cards().len() <= 1)
    ) {
        macro_rules! for_each_allowed_card{
            (($i_offset_0: expr, $($i_offset: expr,)*), $stichseq: expr) => {{
                let epi = epi_current.wrapping_add($i_offset_0);
                let card = debug_verify_eq!(
                    ahand[epi].cards(),
                    &rules.all_allowed_cards($stichseq, &ahand[epi])
                )[0];
                //ahand[epi].play_card(card); // not necessary
                let output = $stichseq.zugeben_and_restore(
                    card,
                    rules,
                    |stichseq| {for_each_allowed_card!(($($i_offset,)*), stichseq)}
                );
                //ahand[epi].add_card(card); // not necessary
                output
            }};
            ((), $stichseq: expr) => {{
                let unregisterstich = rulestatecache.register_stich(
                    unwrap!($stichseq.completed_stichs().last()),
                    $stichseq.current_stich().first_playerindex(),
                );
                let output = foreachsnapshot.final_output(
                    SStichSequenceGameFinished::new($stichseq),
                    rulestatecache,
                );
                rulestatecache.unregister_stich(unregisterstich);
                output
            }};
        }
        match stichseq.current_stich().size() {
            0 => for_each_allowed_card!((0, 1, 2, 3,), stichseq),
            1 => for_each_allowed_card!((0, 1, 2,), stichseq),
            2 => for_each_allowed_card!((0, 1,), stichseq),
            3 => for_each_allowed_card!((0,), stichseq),
            n_stich_size => {
                assert_eq!(n_stich_size, 4);
                for_each_allowed_card!((), stichseq)
            },
        }
    } else {
        foreachsnapshot.pruned_output(stichseq, ahand, rulestatecache).unwrap_or_else(|| {
            let mut veccard_allowed = rules.all_allowed_cards(stichseq, &ahand[epi_current]);
            func_filter_allowed_cards(stichseq, &mut veccard_allowed);
            // TODO? use equivalent card optimization
            foreachsnapshot.combine_outputs(
                epi_current,
                veccard_allowed.into_iter().map(|card| {
                    ahand[epi_current].play_card(card);
                    let output = stichseq.zugeben_and_restore(card, rules, |stichseq| {
                        macro_rules! next_step {() => {explore_snapshots_internal(
                            ahand,
                            rules,
                            rulestatecache,
                            stichseq,
                            func_filter_allowed_cards,
                            foreachsnapshot,
                            snapshotvisualizer,
                        )}}
                        if stichseq.current_stich().is_empty() {
                            let unregisterstich = rulestatecache.register_stich(
                                unwrap!(stichseq.completed_stichs().last()),
                                stichseq.current_stich().first_playerindex(),
                            );
                            let output = next_step!();
                            rulestatecache.unregister_stich(unregisterstich);
                            output
                        } else {
                            next_step!()
                        }
                    });
                    ahand[epi_current].add_card(card);
                    (card, output)
                })
            )
        })
    };
    snapshotvisualizer.end_snapshot(&output);
    output
}

#[derive(Clone, new)]
pub struct SMinReachablePayoutBase<'rules, Pruner> {
    rules: &'rules dyn TRules,
    epi: EPlayerIndex,
    tpln_stoss_doubling: (usize, usize),
    n_stock: isize,
    phantom: std::marker::PhantomData<Pruner>,
}
impl<'rules, Pruner> SMinReachablePayoutBase<'rules, Pruner> {
    pub fn new_from_game(game: &'rules SGame) -> Self {
        Self::new(
            game.rules.as_ref(),
            unwrap!(game.current_playable_stich().current_playerindex()),
            /*tpln_stoss_doubling*/stoss_and_doublings(&game.vecstoss, &game.doublings),
            game.n_stock,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SPerMinMaxStrategy<T> {
    pub t_min: T,
    pub t_selfish_min: T,
    pub t_selfish_max: T,
    pub t_max: T,
}

pub type SMinMax = SPerMinMaxStrategy<EnumMap<EPlayerIndex, isize>>;

impl SMinMax {
    fn new_final(an_payout: EnumMap<EPlayerIndex, isize>) -> Self {
        Self {
            t_min: an_payout.explicit_clone(),
            t_selfish_min: an_payout.explicit_clone(),
            t_selfish_max: an_payout.explicit_clone(),
            t_max: an_payout.explicit_clone(),
        }
    }
}

impl<Pruner: TPruner> TForEachSnapshot for SMinReachablePayoutBase<'_, Pruner> {
    type Output = SMinMax;

    fn final_output(&self, slcstich: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output {
        SMinMax::new_final(self.rules.payout_with_cache(slcstich, self.tpln_stoss_doubling, self.n_stock, rulestatecache))
    }

    fn pruned_output(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<Self::Output> {
        Pruner::pruned_output(self, stichseq, ahand, rulestatecache)
    }

    fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
        &self,
        epi_card: EPlayerIndex,
        ittplcardoutput: ItTplCardOutput,
    ) -> Self::Output {
        let itminmax = ittplcardoutput.map(|(_card, minmax)| minmax);
        unwrap!(if self.epi==epi_card {
            itminmax.reduce(mutate_return!(|minmax_acc, minmax| {
                // self.epi can always play as good as possible
                let play_best = |an_payout_acc, an_payout_new: &EnumMap<EPlayerIndex, isize>| {
                    assign_max_by_key(
                        an_payout_acc,
                        an_payout_new.explicit_clone(),
                        |an_payout| an_payout[self.epi],
                    );
                };
                play_best(&mut minmax_acc.t_min, &minmax.t_min);
                play_best(&mut minmax_acc.t_selfish_min, &minmax.t_selfish_min);
                play_best(&mut minmax_acc.t_selfish_max, &minmax.t_selfish_max);
                play_best(&mut minmax_acc.t_max, &minmax.t_max);
            }))
        } else {
            // other players may play inconveniently for epi_stich
            itminmax.reduce(mutate_return!(|minmax_acc, minmax| {
                assign_min_by_key(
                    &mut minmax_acc.t_min,
                    minmax.t_min.explicit_clone(),
                    |an_payout| an_payout[self.epi],
                );
                assign_better(
                    &mut minmax_acc.t_selfish_min,
                    minmax.t_selfish_min.explicit_clone(),
                    |an_payout_lhs, an_payout_rhs| {
                        match an_payout_lhs[epi_card].cmp(&an_payout_rhs[epi_card]) {
                            Ordering::Less => false,
                            Ordering::Equal => an_payout_lhs[self.epi] < an_payout_rhs[self.epi],
                            Ordering::Greater => true,
                        }
                    },
                );
                assign_better(
                    &mut minmax_acc.t_selfish_max,
                    minmax.t_selfish_max.explicit_clone(),
                    |an_payout_lhs, an_payout_rhs| {
                        match an_payout_lhs[epi_card].cmp(&an_payout_rhs[epi_card]) {
                            Ordering::Less => false,
                            Ordering::Equal => an_payout_lhs[self.epi] > an_payout_rhs[self.epi],
                            Ordering::Greater => true,
                        }
                    },
                );
                assign_max_by_key(
                    &mut minmax_acc.t_max,
                    minmax.t_max.explicit_clone(),
                    |an_payout| an_payout[self.epi],
                );

            }))
        })
    }
}

pub type SMinReachablePayout<'rules> = SMinReachablePayoutBase<'rules, SPrunerNothing>;
pub type SMinReachablePayoutLowerBoundViaHint<'rules> = SMinReachablePayoutBase<'rules, SPrunerViaHint>;

pub trait TPruner : Sized {
    fn pruned_output(params: &SMinReachablePayoutBase<'_, Self>, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<SMinMax>;
}

pub struct SPrunerNothing;
impl TPruner for SPrunerNothing {
    fn pruned_output(_params: &SMinReachablePayoutBase<'_, Self>, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>, _rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
        None
    }
}

pub struct SPrunerViaHint;
impl TPruner for SPrunerViaHint {
    fn pruned_output(params: &SMinReachablePayoutBase<'_, Self>, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
        let mapepion_payout = params.rules.payouthints(stichseq, ahand, rulestatecache).map(|payouthint| {
            payouthint
                .lower_bound()
                .as_ref()
                .map(|payoutinfo|
                    payoutinfo.payout_including_stock(params.n_stock, params.tpln_stoss_doubling)
                )
        });
        if_then_some!(
            mapepion_payout.iter().all(Option::is_some) && 0<unwrap!(mapepion_payout[params.epi]),
            SMinMax::new_final(mapepion_payout.map(|opayout| unwrap!(opayout)))
        )
    }
}
