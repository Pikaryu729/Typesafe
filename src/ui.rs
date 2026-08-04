use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
};

use crate::app::{App, MenuItem, RaceSetupItem, Screen};
use crate::storage::Session;

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
        Screen::RaceSetup => race_setup(frame, sections[1], app),
        Screen::RaceLobby => race_lobby(frame, sections[1], app),
        Screen::Typing => typing_card(frame, sections[1], app),
        Screen::Finished => results(frame, sections[1], app),
        Screen::History => history(frame, sections[1], app),
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
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
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
    let race_style = if app.menu_item == MenuItem::Race {
        Style::default()
            .fg(Color::Rgb(82, 201, 169))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(145, 166, 200))
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{}  Race with an invite code",
            if app.menu_item == MenuItem::Race {
                "›"
            } else {
                " "
            }
        ))
        .style(race_style),
        rows[6],
    );
    let history_style = if app.menu_item == MenuItem::History {
        Style::default()
            .fg(Color::Rgb(255, 211, 126))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(145, 166, 200))
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{}  View typing history",
            if app.menu_item == MenuItem::History {
                "›"
            } else {
                " "
            }
        ))
        .style(history_style),
        rows[7],
    );
}

fn race_setup(frame: &mut Frame, area: Rect, app: &App) {
    let card = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(49, 62, 86)))
        .title(Span::styled(
            "  INVITE-CODE RACE  ",
            Style::default()
                .fg(Color::Rgb(255, 190, 92))
                .add_modifier(Modifier::BOLD),
        ));
    let inner = card.inner(area).inner(Margin {
        horizontal: 4,
        vertical: 2,
    });
    frame.render_widget(card, area);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(2),
        ])
        .split(inner);
    frame.render_widget(
        Paragraph::new(
            "Connect through a TypeSafe race relay. Both racers receive the same prompt.",
        )
        .style(Style::default().fg(Color::Rgb(130, 145, 170)))
        .wrap(Wrap { trim: false }),
        rows[0],
    );
    let host_selected = app.race_setup_item == RaceSetupItem::Host;
    frame.render_widget(
        Paragraph::new(format!(
            "{}  Host a new race",
            if host_selected { "›" } else { " " }
        ))
        .style(selection_style(host_selected)),
        rows[1],
    );
    let join_selected = app.race_setup_item == RaceSetupItem::Join;
    let join_label = if join_selected {
        format!(
            "›  Join with code: {}",
            if app.race_code.is_empty() {
                "______"
            } else {
                &app.race_code
            }
        )
    } else {
        "   Join with code".into()
    };
    frame.render_widget(
        Paragraph::new(join_label).style(selection_style(join_selected)),
        rows[2],
    );
    let detail = app
        .race_status
        .as_deref()
        .unwrap_or("Use ↑↓ or Tab to choose, then Enter.");
    frame.render_widget(
        Paragraph::new(detail).style(Style::default().fg(Color::Rgb(145, 166, 200))),
        rows[3],
    );
}

fn race_lobby(frame: &mut Frame, area: Rect, app: &App) {
    let card = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(82, 201, 169)))
        .title("  RACE LOBBY  ");
    let inner = card.inner(area).inner(Margin {
        horizontal: 4,
        vertical: 3,
    });
    frame.render_widget(card, area);
    let status = app
        .race_status
        .as_deref()
        .unwrap_or("Waiting for race details…");
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(Span::styled(
                "Your invite code",
                Style::default().fg(Color::Rgb(130, 145, 170)),
            )),
            Line::from(Span::styled(
                &app.race_code,
                Style::default()
                    .fg(Color::Rgb(255, 211, 126))
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(status),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Rgb(215, 224, 242))),
        inner,
    );
}

fn selection_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Rgb(255, 211, 126))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(215, 224, 242))
    }
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
    let encouragement = if let Some(opponent) = &app.opponent {
        format!(
            "Opponent: {} chars · {}% accuracy{}",
            opponent.characters,
            opponent.accuracy,
            opponent
                .wpm
                .map_or_else(String::new, |wpm| format!(" · {wpm} WPM")),
        )
    } else {
        "Keep your rhythm — accuracy first.".into()
    };
    frame.render_widget(
        Paragraph::new(encouragement).style(Style::default().fg(Color::Rgb(130, 145, 170))),
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

fn history(frame: &mut Frame, area: Rect, app: &App) {
    let card = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(49, 62, 86)))
        .title(Span::styled(
            "  TYPING HISTORY  ",
            Style::default()
                .fg(Color::Rgb(255, 190, 92))
                .add_modifier(Modifier::BOLD),
        ));
    let inner = card.inner(area);
    frame.render_widget(card, area);
    let content = inner.inner(Margin {
        horizontal: 3,
        vertical: 1,
    });
    if app.sessions().is_empty() {
        frame.render_widget(
            Paragraph::new("No saved sessions yet. Complete a sprint to build your history.")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::Rgb(130, 145, 170))),
            content,
        );
        return;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(8),
        ])
        .split(content);
    let sessions = app.sessions();
    let total_characters: usize = sessions.iter().map(|session| session.characters).sum();
    let average_wpm = sessions
        .iter()
        .map(|session| session.wpm as usize)
        .sum::<usize>()
        / sessions.len();
    frame.render_widget(
        Paragraph::new(format!(
            "{} sessions     {} avg WPM     {} characters",
            sessions.len(),
            average_wpm,
            total_characters
        ))
        .style(
            Style::default()
                .fg(Color::Rgb(215, 224, 242))
                .add_modifier(Modifier::BOLD),
        ),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from("Average WPM by week — last year"),
            Line::from(average_wpm_trend(sessions)),
        ]))
        .style(Style::default().fg(Color::Rgb(130, 145, 170))),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(heatmap(sessions)).style(Style::default().fg(Color::Rgb(145, 166, 200))),
        rows[2],
    );
    if let Some(error) = &app.storage_error {
        frame.render_widget(
            Paragraph::new(format!("Could not save the latest session: {error}"))
                .style(Style::default().fg(Color::Rgb(255, 113, 113))),
            rows[2],
        );
    }
}

fn average_wpm_trend(sessions: &[Session]) -> String {
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        / 86_400;
    let current_week = week_start(today);
    let first_week = current_week - 51 * 7;
    let weekly_totals = sessions.iter().fold(
        BTreeMap::<i64, (usize, usize)>::new(),
        |mut totals, session| {
            let week = week_start(session.started_at / 86_400);
            if week >= first_week && week <= current_week {
                let entry = totals.entry(week).or_default();
                entry.0 += session.wpm as usize;
                entry.1 += 1;
            }
            totals
        },
    );
    let averages = (0..52)
        .map(|offset| {
            weekly_totals
                .get(&(first_week + offset * 7))
                .map(|(total, count)| total / count)
        })
        .collect::<Vec<_>>();
    let maximum = averages.iter().flatten().copied().max().unwrap_or(1).max(1);
    let minimum = averages.iter().flatten().copied().min().unwrap_or(0);
    let range = maximum.saturating_sub(minimum).max(1);
    averages
        .iter()
        .map(|average| {
            average.map_or('·', |wpm| {
                BARS[(wpm.saturating_sub(minimum) * (BARS.len() - 1)) / range]
            })
        })
        .collect()
}

fn heatmap(sessions: &[Session]) -> Text<'static> {
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        / 86_400;
    let current_week = week_start(today);
    let first_week = current_week - 51 * 7;
    let counts = sessions
        .iter()
        .fold(BTreeMap::<i64, usize>::new(), |mut counts, session| {
            *counts.entry(session.started_at / 86_400).or_default() += 1;
            counts
        });
    let maximum = counts.values().copied().max().unwrap_or(1);
    let mut lines = vec![Line::from(Span::styled(
        "Session activity — last year",
        Style::default()
            .fg(Color::Rgb(215, 224, 242))
            .add_modifier(Modifier::BOLD),
    ))];
    for weekday in 0..7 {
        let label = match weekday {
            1 => "Mon ",
            3 => "Wed ",
            5 => "Fri ",
            _ => "    ",
        };
        let mut spans = vec![Span::raw(label)];
        for week in 0..52 {
            let day = first_week + week * 7 + weekday;
            let count = counts.get(&day).copied().unwrap_or(0);
            let color = match count {
                0 => Color::Rgb(31, 39, 56),
                _ if count * 3 <= maximum => Color::Rgb(49, 92, 86),
                _ if count * 3 <= maximum * 2 => Color::Rgb(61, 139, 120),
                _ => Color::Rgb(82, 201, 169),
            };
            spans.push(Span::styled(" ", Style::default().bg(color)));
        }
        lines.push(Line::from(spans));
    }
    Text::from(lines)
}

fn week_start(day: i64) -> i64 {
    day - (day + 4).rem_euclid(7)
}

fn footer(frame: &mut Frame, area: Rect, app: &App) {
    let help = match app.screen {
        Screen::Menu => "↑↓  choose     Space  toggle     ←→  time     Enter  start     q  quit",
        Screen::RaceSetup => "↑↓ / Tab  choose     Enter  continue     Esc  return to setup",
        Screen::RaceLobby => "Esc  cancel race     q  quit",
        Screen::Typing => "Backspace  correct a typo     Esc  return to setup",
        Screen::Finished => "r  setup again     q  quit",
        Screen::History => "Esc / h  return to setup     q  quit",
    };
    frame.render_widget(
        Paragraph::new(help)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Rgb(95, 109, 133))),
        area,
    );
}
