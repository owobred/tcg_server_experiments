use std::sync::Arc;

struct Matchmaker<T>
where
    T: Clone,
{
    _player_id: std::marker::PhantomData<T>,
}

impl<T> Matchmaker<T>
where
    T: Clone,
{
    pub fn add_server(self: Arc<Self>, info: ServerInfo) {
        todo!()
    }

    pub fn add_player_to_pool(self: Arc<Self>, info: PlayerInfo<T>) -> MatchmakerPlayerHandle<T> {
        MatchmakerPlayerHandle {
            matchmaker: self,
            player_info: info,
        }
    }

    pub fn remove_player_from_pool(self: Arc<Self>, player_id: T) {
        todo!()
    }

    fn find_best_match(self: Arc<Self>, info: PlayerInfo<T>) -> Option<PotentialMatchup<T>> {
        todo!()
    }

    fn score_matchup(self: Arc<Self>) {}

    fn find_best_server_for_match(self: Arc<Self>, matchup: PotentialMatchup<T>) -> Match<T> {
        todo!()
    }
}

struct PotentialMatchup<T>
where
    T: Clone,
{
    player: T,
    opponent: T,
}

struct Match<T>
where
    T: Clone,
{
    player: T,
    opponent: T,
    server: ServerInfo,
}

struct PlayerInfo<T>
where
    T: Clone,
{
    id: T,
    elo: usize,
}

struct MatchmakerPlayerHandle<T>
where
    T: Clone,
{
    matchmaker: Arc<Matchmaker<T>>,
    player_info: PlayerInfo<T>,
}

impl<T> Drop for MatchmakerPlayerHandle<T>
where
    T: Clone,
{
    fn drop(&mut self) {
        self.matchmaker
            .clone()
            .remove_player_from_pool(self.player_info.id.to_owned())
    }
}

struct ServerInfo {
    max_players: usize,
    current_players: usize,
    state: ServerState,
}

impl ServerInfo {
    pub fn accepting_players(&self) -> bool {
        self.current_players < self.max_players && self.state == ServerState::Normal
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ServerState {
    Startup,
    Normal,
    Draining,
}
