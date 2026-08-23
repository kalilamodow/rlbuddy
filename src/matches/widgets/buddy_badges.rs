use crate::matches::{MatchPlayer, MatchType};

pub struct BuddyBadge {
    pub badge: String,
    pub detail_text: String,
}

fn bb_sprout(other_player: &MatchPlayer, matches: &Vec<MatchType<'_>>) -> Option<BuddyBadge> {
    let not_played_together = matches.iter().all(|m| match m {
        MatchType::Old(o) => o
            .players
            .iter()
            .all(|p| p.player_id != other_player.data.platform_id),
        MatchType::Session(s) => s
            .players
            .iter()
            .all(|p| p.data.platform_id != other_player.data.platform_id),
    });

    not_played_together.then(|| BuddyBadge {
        badge: "🌱".into(),
        detail_text: "This is your first game together!".to_string(),
    })
}

fn bb_bffs(other_player: &MatchPlayer, matches: &Vec<MatchType<'_>>) -> Option<BuddyBadge> {
    let games_won_together = matches
        .iter()
        .filter(|m| match m {
            MatchType::Old(o) => {
                o.winner == m.our_team()
                    && o.players.iter().any(|p| {
                        p.player_id == other_player.data.platform_id && p.team == m.our_team()
                    })
            }
            MatchType::Session(s) => {
                s.finish
                    .as_ref()
                    .is_some_and(|f| f.winner == Some(m.our_team()))
                    && s.players.iter().any(|p| {
                        p.data.platform_id == other_player.data.platform_id
                            && p.data.team == m.our_team()
                    })
            }
        })
        .count();

    let icon = match games_won_together {
        0 => return None,
        1..3 => '😎',
        3..6 => '✨',
        6..10 => '🫂',
        10.. => '🥰',
    };

    Some(BuddyBadge {
        badge: format!("{icon} {games_won_together}"),
        detail_text: format!("{games_won_together} game(s) won together!"),
    })
}

fn bb_rivals(other_player: &MatchPlayer, matches: &Vec<MatchType<'_>>) -> Option<BuddyBadge> {
    let (wins, losses): (usize, usize) = matches
        .iter()
        .filter_map(|m| match m {
            MatchType::Old(o) => {
                if o.players
                    .iter()
                    .any(|p| p.player_id == other_player.data.platform_id && p.team != m.our_team())
                {
                    Some(if o.winner == o.our_team() {
                        (1, 0)
                    } else {
                        (0, 1)
                    })
                } else {
                    None
                }
            }
            MatchType::Session(s) => {
                if s.players.iter().any(|p| {
                    p.data.platform_id == other_player.data.platform_id
                        && p.data.team != m.our_team()
                }) && let Some(winner) = s.finish.as_ref().map(|f| f.winner)
                {
                    Some(if winner == Some(m.our_team()) {
                        (1, 0)
                    } else {
                        (0, 1)
                    })
                } else {
                    None
                }
            }
        })
        // fold is so cool
        .fold((0, 0), |(wins, losses), (w, l)| (wins + w, losses + l));

    let games_against_eachother = wins + losses;

    let icon = match games_against_eachother {
        0 => return None,
        ..4 => '⚔',
        4.. => '🩸',
    };

    Some(BuddyBadge {
        badge: format!("{icon} {wins}-{losses}"),
        detail_text: format!(
            "{games_against_eachother} game(s) against eachother. (W/L: {wins}/{losses})"
        ),
    })
}

pub fn get_badges(other_player: &MatchPlayer, matches: &Vec<MatchType<'_>>) -> Vec<BuddyBadge> {
    vec![
        bb_sprout(other_player, matches),
        bb_bffs(other_player, matches),
        bb_rivals(other_player, matches),
    ]
    .into_iter()
    .flatten()
    .collect()
}
