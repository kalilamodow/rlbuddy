use egui_plot::PlotPoint;

use crate::{
    common::{
        ReadWriteStateHandle, ReadonlyStateHandle,
        channel::{Receiver, Sender},
    },
    core::app::{Service, ServiceWithUi},
    matches::{MatchInfo, MatchType, MatchesService, MatchesServiceState},
    my_stats::widget::MyStatsWidget,
    rocket_league::Playlist,
    stats_api::{RLEvent, StatsApi},
};
use std::{borrow::Cow, collections::HashSet, rc::Rc, time::SystemTime};

#[derive(Debug, Default)]
pub struct WholeSessionStats {
    pub games: u64,
    pub goals: u64,
    pub assists: u64,
    pub saves: u64,
    pub shots: u64,
    pub demos: u64,
}

#[derive(Default)]
pub enum MmrGraphState {
    #[default]
    NoPlaylist,
    NotEnoughData,
    // these are "put back together" in the x-axis formatter in the widget
    // need a raw PlotPoint vec for egui_plot
    Graph((Vec<PlotPoint>, Vec<Option<SystemTime>>)),
}

pub enum StreakType {
    Win,
    Loss,
}

pub struct StreakStats {
    pub count: usize,
    pub cat: StreakType, // meow
}

pub struct WinLossStats {
    pub wins: usize,
    pub losses: usize,
}

#[derive(Default)]
pub struct MyStatsServiceState {
    pub selected_graph_playlist: Option<Playlist>,
    pub choosable_playlists: Vec<Playlist>,
    pub graph: MmrGraphState,
    pub number_of: WholeSessionStats,
    // probably none if no games
    pub streak: Option<StreakStats>,
    pub wl: Option<WinLossStats>,
}

pub enum MyStatsServiceCommand {
    SetSelectedGraphPlaylist(Playlist),
}

pub struct MyStatsService {
    stats_api: Receiver<Rc<RLEvent>>,
    matches_state: ReadonlyStateHandle<MatchesServiceState>,
    state: ReadWriteStateHandle<MyStatsServiceState>,
    receiver: Receiver<MyStatsServiceCommand>,
}

impl MyStatsService {
    pub fn new(stats_api: &mut StatsApi, matches_service: &MatchesService) -> Self {
        let svc = Self {
            stats_api: stats_api.subscribe(),
            matches_state: matches_service.state_handle(),
            state: ReadWriteStateHandle::new(MyStatsServiceState::default()),
            receiver: Receiver::new(),
        };

        svc.update_choosable_playlists();
        svc.update_longtime_stats();
        svc.update_streak();
        svc.update_wl();
        svc.update_graph_points();

        svc
    }

    fn update_choosable_playlists(&self) {
        let matches_state = self.matches_state.read();
        let mut choosable_playlists: Vec<Playlist> = matches_state
            .prev_matches
            .iter()
            .map(MatchType::playlist)
            .filter_map(Playlist::in_ranked)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        choosable_playlists.sort();

        let mut state = self.state.write();
        state.choosable_playlists = choosable_playlists;
    }

    fn update_longtime_stats(&self) {
        let matches_state = self.matches_state.read();
        let all_stats = matches_state.prev_matches.iter().filter_map(|m| match m {
            MatchType::Session(s) => s
                .players
                .iter()
                .find_map(|p| p.is_local_player.then_some(&p.stats)),
            MatchType::Old(_) => None,
        });

        let totals: WholeSessionStats =
            all_stats.fold(WholeSessionStats::default(), |mut total, stats| {
                total.games += 1;
                total.goals += u64::from(stats.goals);
                total.shots += u64::from(stats.shots);
                total.assists += u64::from(stats.assists);
                total.saves += u64::from(stats.saves);
                total.demos += u64::from(stats.demos);
                total
            });

        let mut state = self.state.write();
        state.number_of = totals;
    }

    fn update_streak(&self) {
        let matches_state = self.matches_state.read();
        let session = || session_matches(&matches_state);

        if session().count() == 0 {
            return;
        }

        let won_last_match = session().next_back().unwrap().is_win();
        let streak = session()
            .rev()
            .take_while(|m| m.is_win() == won_last_match)
            .count();

        let mut state = self.state.write();
        state.streak = Some(StreakStats {
            count: streak,
            cat: if won_last_match {
                StreakType::Win
            } else {
                StreakType::Loss
            },
        });
    }

    fn update_wl(&self) {
        let matches_state = self.matches_state.read();
        let session = || session_matches(&matches_state);

        let total_games = session().count();
        let wins = session()
            .filter(|s| s.finish.as_ref().and_then(|f| f.winner) == Some(s.our_team))
            .count();
        let losses = total_games - wins;

        let mut state = self.state.write();
        state.wl = Some(WinLossStats { wins, losses });
    }

    fn update_graph_points(&self) {
        let mut state = self.state.write();
        let Some(selected_playlist) = state.selected_graph_playlist else {
            state.graph = MmrGraphState::NoPlaylist;
            return;
        };

        let matches = self.matches_state.read();
        // 1 match means its a single point (bad)
        if matches.prev_matches.len() <= 1 {
            state.graph = MmrGraphState::NotEnoughData;
            return;
        }

        let last_match_before_this_session = matches
            .prev_matches
            .iter()
            // since its going backwards, the first one it finds will be the last matching game!
            .rev()
            .find_map(|m| match m {
                MatchType::Old(o) if m.playlist() == selected_playlist => Some(o),
                _ => None,
            })
            .and_then(|last| last.players.iter().find(|p| p.is_local_player()))
            .and_then(|me| me.rank_in_mode.as_ref())
            .map(|r| {
                (
                    PlotPoint {
                        x: 0.0,
                        y: r.mmr as f64,
                    },
                    None,
                )
            });

        let session_points = matches
            .prev_matches
            .iter()
            .filter_map(|m| match m {
                MatchType::Session(s) if m.playlist() == selected_playlist => Some(s),
                _ => None,
            })
            .enumerate()
            .filter_map(|(idx, m)| {
                m.players
                    .iter()
                    .find(|p| p.is_local_player)
                    .and_then(|me| me.skill.as_ref())
                    .map(|sk| sk.get_playlist(selected_playlist).unwrap())
                    .map(|playlist_skill| {
                        (
                            PlotPoint {
                                x: (idx + 1) as f64,
                                y: playlist_skill.mmr as f64,
                            },
                            Some(m.started_at),
                        )
                    })
            });

        let points = last_match_before_this_session
            .into_iter()
            .chain(session_points);

        state.graph = MmrGraphState::Graph(points.unzip());
    }

    pub fn sender(&self) -> Sender<MyStatsServiceCommand> {
        self.receiver.send()
    }

    pub fn state_handle(&self) -> ReadonlyStateHandle<MyStatsServiceState> {
        ReadonlyStateHandle::over(&self.state)
    }
}

impl Service for MyStatsService {
    fn update(&mut self) {
        while let Some(cmd) = self.receiver.try_recv() {
            match cmd {
                MyStatsServiceCommand::SetSelectedGraphPlaylist(pl) => {
                    {
                        let mut state = self.state.write();
                        state.selected_graph_playlist = Some(pl);
                    }

                    self.update_graph_points();
                }
            }
        }

        while let Some(cmd) = self.stats_api.try_recv() {
            if matches!(*cmd, RLEvent::MatchOver(_) | RLEvent::MatchLeft) {
                self.update_choosable_playlists();
                self.update_longtime_stats();
                self.update_streak();
                self.update_wl();
                self.update_graph_points();
            }
        }
    }
}

impl ServiceWithUi for MyStatsService {
    fn panel(&self) -> impl crate::core::app::Panel + 'static {
        MyStatsWidget::new(self)
    }
}

fn session_matches(
    state: &MatchesServiceState,
) -> impl DoubleEndedIterator<Item = &Cow<'_, MatchInfo>> {
    state.prev_matches.iter().filter_map(|m| match m {
        MatchType::Session(s) => Some(s),
        MatchType::Old(_) => None,
    })
}
