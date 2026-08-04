mod app;
mod ui;

use std::{io, time::Duration};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::app::{App, MenuItem, Screen};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run(&mut terminal);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::new();
    loop {
        app.update_clock();
        terminal.draw(|frame| ui::render(frame, &app))?;
        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') if app.screen != Screen::Typing => return Ok(()),
                KeyCode::Esc if app.screen == Screen::Typing => app.return_to_menu(),
                KeyCode::Char('r') if app.screen == Screen::Finished => app.screen = Screen::Menu,
                KeyCode::Up | KeyCode::Char('k') if app.screen == Screen::Menu => {
                    app.menu_item = app.menu_item.previous()
                }
                KeyCode::Down | KeyCode::Char('j') if app.screen == Screen::Menu => {
                    app.menu_item = app.menu_item.next()
                }
                KeyCode::Left | KeyCode::Char('h')
                    if app.screen == Screen::Menu && app.menu_item == MenuItem::Duration =>
                {
                    app.advance_duration(-1)
                }
                KeyCode::Right | KeyCode::Char('l')
                    if app.screen == Screen::Menu && app.menu_item == MenuItem::Duration =>
                {
                    app.advance_duration(1)
                }
                KeyCode::Char(' ') | KeyCode::Enter if app.screen == Screen::Menu => {
                    match app.menu_item {
                        MenuItem::Numbers => app.include_numbers = !app.include_numbers,
                        MenuItem::Capitals => app.include_capitals = !app.include_capitals,
                        MenuItem::Symbols => app.include_symbols = !app.include_symbols,
                        MenuItem::Duration => app.advance_duration(1),
                        MenuItem::Start => app.start(),
                    }
                }
                KeyCode::Backspace if app.screen == Screen::Typing => app.delete_character(),
                KeyCode::Char(character)
                    if app.screen == Screen::Typing && !character.is_control() =>
                {
                    app.type_character(character)
                }
                _ => {}
            }
        }
    }
}
