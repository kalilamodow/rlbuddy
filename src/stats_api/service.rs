use num_enum::TryFromPrimitive;
use serde::Deserialize;
use std::{
    cmp::Ordering,
    fmt,
    io::BufReader,
    net::{SocketAddr, TcpStream},
    str::FromStr,
    sync::mpsc,
    thread,
    time::Duration,
};

use crate::{
    common::eventsource::{EventReceiver, EventSource},
    rocket_league::{Platform, Playlist, Team, asset_to_arena},
};

#[derive(Debug, Deserialize)]
struct StatsApiEvent {
    #[serde(rename = "Event")]
    event: String,
    /// data is a json string
    #[serde(rename = "Data")]
    data: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PlayerStats {
    pub score: u16,
    pub goals: u8,
    pub shots: u8,
    pub assists: u8,
    pub saves: u8,
    pub touches: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiPlayerData {
    name: String,
    /// "Platform identifier in the format Platform|Uid|Splitscreen (e.g. "Steam|123|0", "Epic|456|0")."
    primary_id: String,
    team_num: u8,
    shortcut: u8,
    #[serde(flatten)]
    stats: PlayerStats,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiTeamData {
    score: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiPlayerTargetData {
    shortcut: u8,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsApiGameData {
    teams: [StatsApiTeamData; 2],
    arena: String,
    #[serde(rename = "bOvertime")]
    overtime: bool,
    #[serde(rename = "bReplay")]
    replay: bool,
    target: Option<StatsApiPlayerTargetData>,
    playlist_id: u8,
}

impl StatsApiGameData {
    fn scores(&self) -> TeamScores {
        TeamScores {
            blue: self.teams[0].score,
            orange: self.teams[1].score,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct UpdateStateEventData {
    players: Vec<StatsApiPlayerData>,
    game: StatsApiGameData,
}

#[derive(Debug, Default, Clone)]
pub struct TeamScores {
    pub blue: u8,
    pub orange: u8,
}

impl TeamScores {
    pub fn guess_winner(&self) -> Option<Team> {
        Some(match self.blue.cmp(&self.orange) {
            Ordering::Equal => return None,
            Ordering::Greater => Team::Blue,
            Ordering::Less => Team::Orange,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchState {
    Game,
    Replay,
    Overtime,
}

impl MatchState {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchState::Game => "In game",
            MatchState::Replay => "Watching replay",
            MatchState::Overtime => "In overtime",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct MatchEndedEventData {
    winner_team_num: u8,
}

#[derive(Debug, Clone)]
pub struct PlayerData {
    pub name: String,
    pub platform: Platform,
    pub platform_id: String,
    pub team: Team,
    pub stats: PlayerStats,
}

fn parse_stats_api_player(player: StatsApiPlayerData) -> Option<PlayerData> {
    let parts: Vec<&str> = player.primary_id.split('|').collect();

    if let Ok(platform) = Platform::from_str(parts[0]) {
        Some(PlayerData {
            name: player.name,
            platform,
            platform_id: player.primary_id,
            team: player.team_num.into(),
            stats: player.stats,
        })
    } else {
        None
    }
}

impl fmt::Display for PlayerData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) [{}]",
            self.name, self.platform, self.platform_id
        )
    }
}

#[derive(Debug, Clone)]
pub struct MatchUpdate {
    pub score: TeamScores,
    pub players: Vec<PlayerData>,
    pub arena: &'static str,
    pub state: MatchState,
    pub playlist: Playlist,
}

pub enum RLEvent {
    Update(MatchUpdate),
    MatchStart,
    MatchOver(Team), // winner
    MatchLeft,

    ReplayStart,
    ReplayDone,

    Connected,
    Disconnected,

    OurPlayerId(String),
}

enum ApiUpdate {
    Connected,
    Disconnected,
    Event(StatsApiEvent),
}

pub struct StatsApi {
    event_rx: mpsc::Receiver<ApiUpdate>,
    local_player_id_event_emitted_yet: bool,
    match_created_event_happened: bool,
    publisher: EventSource<RLEvent>,
}

impl StatsApi {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                let connection = loop {
                    match TcpStream::connect("127.0.0.1:49123".parse::<SocketAddr>().unwrap()) {
                        Ok(stream) => break stream,
                        Err(_) => thread::sleep(Duration::from_secs(5)),
                    }
                };

                if event_tx.send(ApiUpdate::Connected).is_err() {
                    println!("[stats api] failed to send conected event, quitting");
                    return;
                }

                let reader = BufReader::new(connection);
                let deserializer = serde_json::Deserializer::from_reader(reader);

                for event in deserializer.into_iter() {
                    let event = match event {
                        Ok(e) => e,
                        Err(error) => {
                            println!("deserialize error: {error:?}");
                            continue;
                        }
                    };

                    if event_tx.send(ApiUpdate::Event(event)).is_err() {
                        println!("[stats api] failed to send event, quitting");
                        return;
                    }
                }

                // disconnected
                if event_tx.send(ApiUpdate::Disconnected).is_err() {
                    println!("[stats api] failed to send disconnected event, quitting");
                    return;
                }
            }
        });

        StatsApi {
            event_rx,
            local_player_id_event_emitted_yet: false,
            match_created_event_happened: false,
            publisher: EventSource::new(),
        }
    }

    pub fn subscribe(&mut self) -> EventReceiver<RLEvent> {
        self.publisher.subscribe()
    }

    pub fn update(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            let rl_event = match event {
                ApiUpdate::Connected => Some(RLEvent::Connected),
                ApiUpdate::Disconnected => Some(RLEvent::Disconnected),
                ApiUpdate::Event(evt) => self.on_stats_api_event(&evt),
            };

            if let Some(e) = rl_event {
                self.publisher.publish(e);
            }
        }
    }

    fn on_stats_api_event(&mut self, event: &StatsApiEvent) -> Option<RLEvent> {
        match event.event.as_str() {
            "UpdateState" => {
                let data: UpdateStateEventData = serde_json::from_str(&event.data).unwrap();

                if !self.local_player_id_event_emitted_yet
                    && let Some(game_target) = data.game.target.as_ref()
                {
                    let target_shortcut = game_target.shortcut;
                    let our_player = data.players.iter().find(|p| p.shortcut == target_shortcut);
                    if let Some(player) = our_player {
                        self.local_player_id_event_emitted_yet = true;
                        return Some(RLEvent::OurPlayerId(player.primary_id.clone()));
                    }
                }

                Some(RLEvent::Update(MatchUpdate {
                    state: if data.game.replay {
                        MatchState::Replay
                    } else if data.game.overtime {
                        MatchState::Overtime
                    } else {
                        MatchState::Game
                    },
                    score: data.game.scores(),
                    arena: asset_to_arena(&data.game.arena).unwrap_or("Unknown"),
                    players: data
                        .players
                        .into_iter()
                        .filter_map(parse_stats_api_player)
                        .collect(),
                    playlist: Playlist::try_from_primitive(data.game.playlist_id).unwrap(),
                }))
            }
            "MatchCreated" => {
                self.match_created_event_happened = true;
                None
            }
            "CountdownBegin" if self.match_created_event_happened => {
                self.match_created_event_happened = false;
                Some(RLEvent::MatchStart)
            }
            "MatchEnded" => {
                let data: MatchEndedEventData = serde_json::from_str(&event.data).unwrap();
                Some(RLEvent::MatchOver(Team::from(data.winner_team_num)))
            }
            "MatchDestroyed" => Some(RLEvent::MatchLeft),
            "GoalReplayStart" => Some(RLEvent::ReplayStart),
            "GoalReplayEnd" => Some(RLEvent::ReplayDone),
            _ => None,
        }
    }
}
