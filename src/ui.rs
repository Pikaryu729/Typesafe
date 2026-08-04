use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
};

use crate::app::{App, MenuItem, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(14, 18, 28))),
        area,
    );
    let width = area.width.min(96);
    let content = Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        width,
        ..area
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(12),
            Constraint::Length(3),
        ])
        .split(content.inner(Margin {
            horizontal: 2,
            vertical: 1,
        }));
    header(frame, sections[0], app);
    match app.screen {
        Screen::Menu => menu(frame, sections[1], app),
        Screen::Typing => typing_card(frame, sections[1], app),
        Screen::Finished => results(frame, sections[1], app),
    }
    footer(frame, sections[2], app);
}

fn header(frame: &mut Frame, area: Rect, app: &App) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "TYPE",
            Style::default()
                .fg(Color::Rgb(255, 190, 92))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "SAFE",
            Style::default()
                .fg(Color::Rgb(225, 232, 245))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  •  focused typing practice",
            Style::default().fg(Color::Rgb(130, 145, 170)),
        ),
    ]))
    .alignment(Alignment::Left);
    frame.render_widget(title, area);
    let time = if app.screen == Screen::Typing {
        app.duration()
            .map_or_else(|| "∞".into(), |_| format!("{:02}s", app.seconds_left()))
    } else {
        app.duration_label()
    };
    frame.render_widget(
        Paragraph::new(time).alignment(Alignment::Right).style(
            Style::default()
                .fg(Color::Rgb(145, 166, 200))
                .add_modifier(Modifier::BOLD),
        ),
        area,
    );
}

fn menu(frame: &mut Frame, area: Rect, app: &App) {
    let card = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(49, 62, 86)))
        .title(Span::styled(
            "  PRACTICE SETUP  ",
            Style::default()
                .fg(Color::Rgb(255, 190, 92))
                .add_modifier(Modifier::BOLD),
        ));
    let inner = card.inner(area);
    frame.render_widget(card, area);
    let content = inner.inner(Margin {
        horizontal: 4,
        vertical: 2,
    });
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(content);
    frame.render_widget(
        Paragraph::new("Choose what appears in the random word stream.")
            .style(Style::default().fg(Color::Rgb(130, 145, 170))),
        rows[0],
    );
    setting_row(
        frame,
        rows[1],
        "Include numbers",
        app.include_numbers,
        app.menu_item == MenuItem::Numbers,
    );
    setting_row(
        frame,
        rows[2],
        "Include capital letters",
        app.include_capitals,
        app.menu_item == MenuItem::Capitals,
    );
    setting_row(
        frame,
        rows[3],
        "Include symbols",
        app.include_symbols,
        app.menu_item == MenuItem::Symbols,
    );
    let duration_style = if app.menu_item == MenuItem::Duration {
        Style::default()
            .fg(Color::Rgb(255, 211, 126))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(215, 224, 242))
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{}  Duration:  ‹ {} ›",
            if app.menu_item == MenuItem::Duration {
                "›"
            } else {
                " "
            },
            app.duration_label()
        ))
        .style(duration_style),
        rows[4],
    );
    let start_style = if app.menu_item == MenuItem::Start {
        Style::default()
            .fg(Color::Rgb(14, 18, 28))
            .bg(Color::Rgb(82, 201, 169))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Rgb(82, 201, 169))
            .add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new("  Start practice  ").style(start_style),
        rows[5],
    );
}

fn setting_row(frame: &mut Frame, area: Rect, label: &str, enabled: bool, selected: bool) {
    let toggle = if enabled { "[x]" } else { "[ ]" };
    let marker = if selected { "›" } else { " " };
    let style = if selected {
        Style::default()
            .fg(Color::Rgb(255, 211, 126))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(215, 224, 242))
    };
    frame.render_widget(
        Paragraph::new(format!("{marker}  {toggle}  {label}")).style(style),
        area,
    );
}

fn typing_card(frame: &mut Frame, area: Rect, app: &App) {
    let card = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(49, 62, 86)))
        .title(Span::styled(
            "  TODAY'S SPRINT  ",
            Style::default()
                .fg(Color::Rgb(255, 190, 92))
                .add_modifier(Modifier::BOLD),
        ));
    let inner = card.inner(area);
    frame.render_widget(card, area);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(inner.inner(Margin {
            horizontal: 3,
            vertical: 2,
        }));
    frame.render_widget(
        Paragraph::new("Keep your rhythm — accuracy first.")
            .style(Style::default().fg(Color::Rgb(130, 145, 170))),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(colored_prompt(&app.typed, &app.prompt))
            .wrap(Wrap { trim: false })
            .style(Style::default().add_modifier(Modifier::BOLD)),
        chunks[1],
    );
    let progress = app
        .duration()
        .map_or(0.0, |seconds| app.elapsed.as_secs_f64() / seconds as f64);
    frame.render_widget(
        Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(Color::Rgb(82, 201, 169))
                    .bg(Color::Rgb(31, 39, 56)),
            )
            .ratio(progress.min(1.0))
            .label(if app.duration().is_some() {
                format!("  {}% of session", (progress * 100.0) as u16)
            } else {
                format!("  {} characters", app.typed.chars().count())
            })
            .use_unicode(true),
        chunks[2],
    );
}

fn colored_prompt(typed: &str, prompt: &str) -> Text<'static> {
    let target: Vec<char> = prompt.chars().collect();
    let entered: Vec<char> = typed.chars().collect();
    let mut spans = Vec::with_capacity(target.len());
    for (index, character) in target.iter().enumerate() {
        let style = match entered.get(index) {
            Some(found) if found == character => Style::default().fg(Color::Rgb(214, 234, 228)),
            Some(_) => Style::default()
                .fg(Color::Rgb(255, 113, 113))
                .add_modifier(Modifier::UNDERLINED),
            None if index == entered.len() => Style::default()
                .fg(Color::Rgb(255, 211, 126))
                .add_modifier(Modifier::SLOW_BLINK),
            None => Style::default().fg(Color::Rgb(104, 118, 142)),
        };
        spans.push(Span::styled(character.to_string(), style));
    }
    Text::from(Line::from(spans))
}

fn results(frame: &mut Frame, area: Rect, app: &App) {
    let card = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(82, 201, 169)))
        .title("  SPRINT COMPLETE  ");
    let inner = card.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(card, area);
    let stats = format!(
        "{} WPM     {}% accuracy     {} characters",
        app.wpm(),
        app.accuracy(),
        app.typed.chars().count()
    );
    frame.render_widget(
        Paragraph::new("Nice work.")
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Rgb(255, 211, 126))
                    .add_modifier(Modifier::BOLD),
            ),
        inner,
    );
    frame.render_widget(
        Paragraph::new(stats)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Rgb(215, 224, 242))),
        inner.inner(Margin {
            horizontal: 0,
            vertical: 3,
        }),
    );
    frame.render_widget(
        Paragraph::new("Press r to try again")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Rgb(130, 145, 170))),
        inner.inner(Margin {
            horizontal: 0,
            vertical: 6,
        }),
    );
}

fn footer(frame: &mut Frame, area: Rect, app: &App) {
    let help = match app.screen {
        Screen::Menu => "↑↓  choose     Space  toggle     ←→  time     Enter  start     q  quit",
        Screen::Typing => "Backspace  correct a typo     Esc  return to setup",
        Screen::Finished => "r  setup again     q  quit",
    };
    frame.render_widget(
        Paragraph::new(help)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Rgb(95, 109, 133))),
        area,
    );
}
