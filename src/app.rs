use std::time::{Duration, Instant};

const WORDS: &[&str] = &[
    "amber", "anchor", "apple", "balance", "breeze", "bright", "canvas", "cedar", "clarity",
    "cloud", "copper", "drift", "ember", "field", "focus", "forest", "gentle", "golden", "harbor",
    "island", "juniper", "kindle", "lantern", "maple", "meadow", "moss", "north", "ocean", "paper",
    "pebble", "quiet", "river", "saffron", "signal", "silver", "spring", "stone", "sunset",
    "thread", "valley", "velvet", "willow", "winter", "wonder",
];
pub const DURATIONS: [Option<u64>; 5] = [Some(15), Some(30), Some(60), Some(120), None];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Menu,
    Typing,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Numbers,
    Capitals,
    Symbols,
    Duration,
    Start,
}

impl MenuItem {
    const ALL: [Self; 5] = [
        Self::Numbers,
        Self::Capitals,
        Self::Symbols,
        Self::Duration,
        Self::Start,
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

pub struct App {
    pub screen: Screen,
    pub menu_item: MenuItem,
    pub include_numbers: bool,
    pub include_capitals: bool,
    pub include_symbols: bool,
    pub duration_index: usize,
    pub typed: String,
    pub prompt: String,
    random_state: u64,
    started_at: Option<Instant>,
    pub elapsed: Duration,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Menu,
            menu_item: MenuItem::Numbers,
            include_numbers: false,
            include_capitals: false,
            include_symbols: false,
            duration_index: 2,
            typed: String::new(),
            prompt: String::new(),
            random_state: 0x9e37_79b9_7f4a_7c15,
            started_at: None,
            elapsed: Duration::ZERO,
        }
    }

    pub fn start(&mut self) {
        self.screen = Screen::Typing;
        self.typed.clear();
        self.prompt.clear();
        self.extend_prompt();
        self.elapsed = Duration::ZERO;
        self.started_at = Some(Instant::now());
    }

    pub fn return_to_menu(&mut self) {
        self.update_elapsed();
        self.started_at = None;
        self.screen = Screen::Menu;
    }

    pub fn update_clock(&mut self) {
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
        self.screen = Screen::Finished;
        self.started_at = None;
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
        self.extend_prompt();
    }

    pub fn delete_character(&mut self) {
        self.typed.pop();
    }

    fn next_random(&mut self) -> u64 {
        self.random_state = self
            .random_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        self.random_state
    }

    fn extend_prompt(&mut self) {
        while self.prompt.chars().count() < self.typed.chars().count() + 360 {
            if !self.prompt.is_empty() {
                self.prompt.push(' ');
            }
            let mut word = WORDS[(self.next_random() as usize) % WORDS.len()].to_owned();
            if self.include_capitals && self.next_random().is_multiple_of(5) {
                word.replace_range(..1, &word[..1].to_uppercase());
            }
            if self.include_numbers && self.next_random().is_multiple_of(6) {
                word.push_str(&format!("{}", self.next_random() % 100));
            }
            if self.include_symbols && self.next_random().is_multiple_of(7) {
                word.push(['!', '?', '#', '@'][(self.next_random() as usize) % 4]);
            }
            self.prompt.push_str(&word);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_accuracy_from_typed_characters() {
        let mut app = App::new();
        app.typed = "The x".into();
        app.prompt = "The best".into();
        assert_eq!(app.accuracy(), 80);
    }

    #[test]
    fn time_is_capped_at_zero() {
        let mut app = App::new();
        app.elapsed = Duration::from_secs(99);
        assert_eq!(app.seconds_left(), 0);
    }

    #[test]
    fn generates_configured_random_words() {
        let mut app = App::new();
        app.include_numbers = true;
        app.include_capitals = true;
        app.include_symbols = true;
        app.extend_prompt();
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
}
