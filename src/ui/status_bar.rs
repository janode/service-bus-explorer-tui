use ratatui::prelude::*;
use ratatui::widgets::*;
use ratatui::Frame;

use crate::app::App;

pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let style = if app.status_is_error {
        Style::default().bg(Color::Red).fg(Color::White)
    } else {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    };

    let left = Span::styled(format!(" {} ", app.status_message), style);

    // Auto-refresh countdown indicator
    let refresh_indicator = if app.auto_refresh_enabled
        && app.management.is_some()
        && app.config.settings.auto_refresh_secs > 0
    {
        if let Some(last) = app.last_refresh {
            let elapsed = last.elapsed().as_secs();
            let remaining = app
                .config
                .settings
                .auto_refresh_secs
                .saturating_sub(elapsed);
            format!(" \u{27F3} {}s ", remaining)
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let refresh_span = Span::styled(
        &refresh_indicator,
        Style::default().bg(Color::DarkGray).fg(Color::Cyan),
    );

    let right_text = match app.focus {
        crate::app::FocusPanel::Tree => "Tree",
        crate::app::FocusPanel::Detail => "Detail",
        crate::app::FocusPanel::Messages => "Messages",
    };
    let right = Span::styled(
        format!(" {} | ? Help ", right_text),
        Style::default().bg(Color::DarkGray).fg(Color::Gray),
    );

    let used = app.status_message.len() as u16
        + 2 // left padding
        + refresh_indicator.len() as u16
        + right_text.len() as u16
        + 12; // right padding + " | ? Help "

    let bar = Line::from(vec![
        left,
        refresh_span,
        Span::styled(
            " ".repeat(area.width.saturating_sub(used) as usize),
            Style::default().bg(Color::DarkGray),
        ),
        right,
    ]);

    frame.render_widget(Paragraph::new(bar), area);
}
