use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use crate::race::{RaceClient, RaceEvent};
use crate::storage::{Session, SessionStore, current_timestamp};

static WORDS: LazyLock<Vec<&str>> = LazyLock::new(|| {
    include_str!("../words.txt")
        .lines()
        .filter(|word| !word.is_empty())
        .collect()
});
pub const DURATIONS: [Option<u64>; 5] = [Some(15), Some(30), Some(60), Some(120), None];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    RaceSetup,
    RaceLobby,
    Typing,
    Finished,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Numbers,
    Capitals,
    Symbols,
    Duration,
    Start,
    Race,
    History,
}

impl MenuItem {
    const ALL: [Self; 7] = [
        Self::Numbers,
        Self::Capitals,
        Self::Symbols,
        Self::Duration,
        Self::Start,
        Self::Race,
        Self::History,
    ];

    pub fn next(self) -> Self {
        let position = Self::ALL.iter().position(|item| *item == self).unwrap_or(0);
        Self::ALL[(position + 1) % Self::ALL.len()]
    }

    pub fn previous(self) -> Self {
        let position = Self::ALL.iter().position(|item| *item == self).unwrap_or(0);
        Self::ALL[(position + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceSetupItem {
    Host,
    Join,
}

impl RaceSetupItem {
    pub fn toggle(self) -> Self {
        match self {
            Self::Host => Self::Join,
            Self::Join => Self::Host,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opponent {
    pub characters: usize,
    pub wpm: Option<u16>,
    pub accuracy: u16,
    pub finished: bool,
}

pub struct App {
    pub screen: Screen,
    pub menu_item: MenuItem,
    pub include_numbers: bool,
    pub include_capitals: bool,
    pub include_symbols: bool,
    pub duration_index: usize,
    pub typed: String,
    pub prompt: String,
    started_at: Option<Instant>,
    session_started_at: Option<i64>,
    pub elapsed: Duration,
    sessions: Vec<Session>,
    store: SessionStore,
    pub storage_error: Option<String>,
    pub race_setup_item: RaceSetupItem,
    pub race_code: String,
    pub race_status: Option<String>,
    pub opponent: Option<Opponent>,
    race_client: Option<RaceClient>,
    race_start_at: Option<std::time::SystemTime>,
}

impl App {
    pub fn new() -> rusqlite::Result<Self> {
        let store = SessionStore::open_default()?;
        let sessions = store.sessions()?;
        Ok(Self {
            screen: Screen::Menu,
            menu_item: MenuItem::Numbers,
            include_numbers: false,
            include_capitals: false,
            include_symbols: false,
            duration_index: 2,
            typed: String::new(),
            prompt: String::new(),
            started_at: None,
            session_started_at: None,
            elapsed: Duration::ZERO,
            sessions,
            store,
            storage_error: None,
            race_setup_item: RaceSetupItem::Host,
            race_code: String::new(),
            race_status: None,
            opponent: None,
            race_client: None,
            race_start_at: None,
        })
    }

    pub fn start(&mut self) {
        self.race_client = None;
        self.race_start_at = None;
        self.opponent = None;
        self.screen = Screen::Typing;
        self.typed.clear();
        self.prompt.clear();
        self.extend_prompt();
        self.elapsed = Duration::ZERO;
        self.started_at = Some(Instant::now());
        self.session_started_at = Some(current_timestamp());
    }

    pub fn open_race_setup(&mut self) {
        self.race_client = None;
        self.race_start_at = None;
        self.screen = Screen::RaceSetup;
        self.race_setup_item = RaceSetupItem::Host;
        self.race_code.clear();
        self.race_status = None;
        self.opponent = None;
    }

    pub fn host_race(&mut self, relay_address: &str) -> Result<(), String> {
        self.prepare_race_prompt();
        self.race_client = Some(
            RaceClient::host(relay_address, self.duration(), self.prompt.clone())
                .map_err(|error| format!("Could not reach relay: {error}"))?,
        );
        self.screen = Screen::RaceLobby;
        self.race_status = Some("Creating invite code…".into());
        Ok(())
    }

    pub fn join_race(&mut self, relay_address: &str) -> Result<(), String> {
        if self.race_code.trim().len() != 6 {
            return Err("Enter the six-character invite code.".into());
        }
        self.race_client = Some(
            RaceClient::join(relay_address, &self.race_code)
                .map_err(|error| format!("Could not reach relay: {error}"))?,
        );
        self.race_code = self.race_code.trim().to_ascii_uppercase();
        self.screen = Screen::RaceLobby;
        self.race_status = Some("Joining race…".into());
        Ok(())
    }

    pub fn poll_race(&mut self) {
        let mut events = Vec::new();
        if let Some(client) = &self.race_client {
            while let Some(event) = client.try_event() {
                events.push(event);
            }
        }
        for event in events {
            match event {
                RaceEvent::Hosted { code } => {
                    self.race_code = code;
                    self.race_status = Some("Share this code. Waiting for your opponent…".into());
                }
                RaceEvent::Start {
                    starts_at_millis,
                    duration_seconds,
                    prompt,
                } => {
                    if let Some(index) = DURATIONS
                        .iter()
                        .position(|duration| *duration == duration_seconds)
                    {
                        self.duration_index = index;
                    }
                    self.prompt = prompt;
                    self.typed.clear();
                    self.elapsed = Duration::ZERO;
                    self.opponent = Some(Opponent {
                        characters: 0,
                        wpm: None,
                        accuracy: 100,
                        finished: false,
                    });
                    self.race_start_at =
                        Some(std::time::UNIX_EPOCH + Duration::from_millis(starts_at_millis));
                    self.race_status = Some("Opponent joined. Starting together…".into());
                }
                RaceEvent::PeerProgress {
                    characters,
                    accuracy,
                } => {
                    if let Some(opponent) = &mut self.opponent {
                        opponent.characters = characters;
                        opponent.accuracy = accuracy;
                    }
                }
                RaceEvent::PeerFinished {
                    characters,
                    wpm,
                    accuracy,
                } => {
                    self.opponent = Some(Opponent {
                        characters,
                        wpm: Some(wpm),
                        accuracy,
                        finished: true,
                    });
                }
                RaceEvent::PeerDisconnected => {
                    if self.screen == Screen::RaceLobby || self.screen == Screen::Typing {
                        self.race_status = Some("Your opponent disconnected.".into());
                    }
                }
                RaceEvent::Error(message) => {
                    self.race_status = Some(format!("Race error: {}", message.replace('-', " ")));
                    if self.screen == Screen::RaceLobby {
                        self.screen = Screen::RaceSetup;
                    }
                }
            }
        }
    }

    pub fn return_to_menu(&mut self) {
        self.update_elapsed();
        self.record_session();
        self.report_race_finish();
        self.started_at = None;
        self.session_started_at = None;
        self.screen = Screen::Menu;
    }

    pub fn update_clock(&mut self) {
        if self.screen == Screen::RaceLobby
            && self
                .race_start_at
                .is_some_and(|start| std::time::SystemTime::now() >= start)
        {
            self.start_race();
        }
        if let Some(start) = self.started_at {
            self.elapsed = start.elapsed();
            if self
                .duration()
                .is_some_and(|seconds| self.elapsed >= Duration::from_secs(seconds))
            {
                self.finish();
            }
        }
    }

    pub fn finish(&mut self) {
        self.update_elapsed();
        self.record_session();
        self.report_race_finish();
        self.screen = Screen::Finished;
        self.started_at = None;
        self.session_started_at = None;
    }

    pub fn update_elapsed(&mut self) {
        if let Some(start) = self.started_at {
            self.elapsed = start.elapsed();
        }
    }

    pub fn seconds_left(&self) -> u64 {
        self.duration()
            .unwrap_or_default()
            .saturating_sub(self.elapsed.as_secs())
    }

    pub fn duration(&self) -> Option<u64> {
        DURATIONS[self.duration_index]
    }

    pub fn duration_label(&self) -> String {
        self.duration()
            .map_or_else(|| "Infinite".into(), |seconds| format!("{seconds} seconds"))
    }

    pub fn advance_duration(&mut self, offset: isize) {
        self.duration_index =
            (self.duration_index as isize + offset).rem_euclid(DURATIONS.len() as isize) as usize;
    }

    pub fn type_character(&mut self, character: char) {
        self.typed.push(character);
        if self.race_client.is_none() {
            self.extend_prompt();
        }
        self.report_race_progress();
    }

    pub fn delete_character(&mut self) {
        self.typed.pop();
        self.report_race_progress();
    }

    fn extend_prompt(&mut self) {
        self.extend_prompt_to(self.typed.chars().count() + 360);
    }

    fn prepare_race_prompt(&mut self) {
        self.prompt.clear();
        self.extend_prompt_to(6_000);
    }

    fn extend_prompt_to(&mut self, target_length: usize) {
        while self.prompt.chars().count() < target_length {
            if !self.prompt.is_empty() {
                self.prompt.push(' ');
            }
            let mut word = WORDS[rand::random_range(0..WORDS.len())].to_owned();
            if self.include_capitals && rand::random_ratio(1, 5) {
                word.replace_range(..1, &word[..1].to_uppercase());
            }
            if self.include_numbers && rand::random_ratio(1, 6) {
                word.push_str(&rand::random_range(0..100).to_string());
            }
            if self.include_symbols && rand::random_ratio(1, 7) {
                word.push(['!', '?', '#', '@'][rand::random_range(0..4)]);
            }
            self.prompt.push_str(&word);
        }
    }

    fn start_race(&mut self) {
        self.screen = Screen::Typing;
        self.started_at = Some(Instant::now());
        self.session_started_at = Some(current_timestamp());
        self.race_start_at = None;
        self.race_status = None;
    }

    fn report_race_progress(&self) {
        if let Some(client) = &self.race_client {
            client.progress(self.typed.chars().count(), self.accuracy());
        }
    }

    fn report_race_finish(&self) {
        if let Some(client) = &self.race_client {
            client.finish(self.typed.chars().count(), self.wpm(), self.accuracy());
        }
    }

    pub fn accuracy(&self) -> u16 {
        let typed = self.typed.chars().collect::<Vec<_>>();
        if typed.is_empty() {
            return 100;
        }
        let correct = typed
            .iter()
            .zip(self.prompt.chars())
            .filter(|(a, b)| *a == b)
            .count();
        ((correct * 100) / typed.len()) as u16
    }

    pub fn wpm(&self) -> u16 {
        let minutes = self.elapsed.as_secs_f64() / 60.0;
        if minutes <= 0.0 {
            0
        } else {
            ((self.typed.chars().count() as f64 / 5.0) / minutes).round() as u16
        }
    }

    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    fn record_session(&mut self) {
        let session = Session {
            started_at: self.session_started_at.unwrap_or_else(current_timestamp),
            duration_seconds: self.elapsed.as_secs(),
            characters: self.typed.chars().count(),
            wpm: self.wpm(),
            accuracy: self.accuracy(),
        };
        match self.store.save(&session) {
            Ok(()) => self.sessions.push(session),
            Err(error) => self.storage_error = Some(error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_accuracy_from_typed_characters() {
        let mut app = App::new().unwrap();
        app.typed = "The x".into();
        app.prompt = "The best".into();
        assert_eq!(app.accuracy(), 80);
    }

    #[test]
    fn time_is_capped_at_zero() {
        let mut app = App::new().unwrap();
        app.elapsed = Duration::from_secs(99);
        assert_eq!(app.seconds_left(), 0);
    }

    #[test]
    fn generates_configured_random_words() {
        let mut app = App::new().unwrap();
        app.include_numbers = true;
        app.include_capitals = true;
        app.include_symbols = true;
        app.start();
        assert!(
            app.prompt
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        assert!(
            app.prompt
                .chars()
                .any(|character| character.is_ascii_uppercase())
        );
        assert!(
            app.prompt
                .chars()
                .any(|character| "!?#@".contains(character))
        );
    }

    #[test]
    fn generates_words_from_the_word_bank() {
        let mut app = App::new().unwrap();
        app.start();

        assert!(
            app.prompt
                .split_whitespace()
                .all(|word| WORDS.contains(&word))
        );
    }

    #[test]
    fn prepares_a_long_shared_prompt_for_a_race() {
        let mut app = App::new().unwrap();
        app.prepare_race_prompt();

        assert!(app.prompt.chars().count() >= 6_000);
    }

    #[test]
    fn hosting_a_race_enters_the_lobby() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        std::thread::spawn(move || crate::race::run_relay(listener).unwrap());
        let mut app = App::new().unwrap();

        app.host_race(&address).unwrap();

        assert_eq!(app.screen, Screen::RaceLobby);
        assert!(app.prompt.chars().count() >= 6_000);
    }
}
