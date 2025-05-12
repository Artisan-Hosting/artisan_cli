use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::io;
use tabled::{
    settings::{
        object::{Columns, Segment},
        Format, Modify,
    },
    Table,
};
use unicode_width::UnicodeWidthStr; // helps account for actual display width

pub fn print_logs(lines: Vec<String>, title: String) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut scroll: u16 = 0;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let text: Vec<Line> = lines.iter().map(|l| Line::raw(l.clone())).collect();

            let paragraph = Paragraph::new(text)
                .block(
                    Block::default()
                        .title(title.clone())
                        .borders(Borders::empty()),
                )
                .scroll((scroll, 0));

            f.render_widget(paragraph, size);
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => scroll += 1,
                    KeyCode::Up => scroll = scroll.saturating_sub(1),
                    KeyCode::PageDown => scroll += 10,
                    KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

pub fn style_table(mut table: &mut Table, status_col: Option<usize>, center: bool) -> Table {
    if center {
        table = table.with(Modify::new(Segment::all()).with(tabled::settings::Alignment::center()));
    } else {
        table = table.with(Modify::new(Segment::all()).with(tabled::settings::Alignment::left()));
    }

    if let Some(col) = status_col {
        table = table.with(
            Modify::new(Columns::single(col)).with(Format::content(|text| {
                pad_and_color(text, std::cmp::max(4, text.len() / 2))
            })),
        );
    }

    table
        .with(tabled::settings::Style::empty())
        .with(Modify::new(Segment::all()).with(tabled::settings::Alignment::center()));
    table.to_owned()
}

pub fn display_table(table: &Table) {
    println!();
    println!("{}", table);
    println!();
}

pub fn pad_and_color(text: &str, width: usize) -> String {
    let display_width = UnicodeWidthStr::width(text);
    let padding = width.saturating_sub(display_width) * 4;
    let left = padding / 2;
    let right = padding - left;

    format!(
        "{}{}{}",
        "".repeat(left),
        match text.to_string().as_str() {
            "Running" => text.green().bold().to_string(),
            "Warning" => text.yellow().bold().to_string(),
            "Stopped" => text.red().bold().to_string(),
            "Status" => text.white().bold().to_string(),
            _ => text.to_string(),
        },
        "".repeat(right)
    )
}

pub fn strip_ansi_codes(s: &str) -> String {
    let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    re.replace_all(s, "").to_string()
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let bytes_f64 = bytes as f64;

    if bytes_f64 >= TB {
        format!("{:.2} TB", bytes_f64 / TB)
    } else if bytes_f64 >= GB {
        format!("{:.2} GB", bytes_f64 / GB)
    } else if bytes_f64 >= MB {
        format!("{:.2} MB", bytes_f64 / MB)
    } else if bytes_f64 >= KB {
        format!("{:.2} KB", bytes_f64 / KB)
    } else {
        format!("{} B", bytes)
    }
}
