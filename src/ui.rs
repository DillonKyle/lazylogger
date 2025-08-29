use crate::app::{App, CurrentScreen, SettingConfig, Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Padding, Paragraph, Scrollbar, Wrap},
};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn ui(frame: &mut Frame, app: &mut App) {
    let background = Block::default().style(Style::default().bg(Theme::default().background));
    frame.render_widget(background, frame.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(frame.area());

    let title_block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default());

    let title = Paragraph::new(Text::styled(
        "LazyLogger",
        Style::default()
            .bg(Theme::default().background)
            .fg(Theme::default().green),
    ))
    .block(title_block);

    frame.render_widget(title, chunks[0]);

    let mut event_block = Block::default()
        .title(" Service Events - (e) to focus ")
        .borders(Borders::ALL);

    if app.viewing_logs
        && !app.service_events.items.is_empty()
        && matches!(app.current_screen, CurrentScreen::Main)
    {
        event_block = Block::default()
            .title(" Service Events - (e) to unfocus - (r) to refresh ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Theme::default().green));
    } else if app.viewing_logs
        && app.service_events.items.is_empty()
        && matches!(app.current_screen, CurrentScreen::Main)
    {
        event_block = Block::default()
            .title(" Service Events - (e) to unfocus ")
            .borders(Borders::ALL)
            .style(Style::default().fg(Theme::default().green));
    }

    let event_items: Vec<ListItem> = app
        .service_events
        .items
        .iter()
        .map(|item| {
            ListItem::new(Line::from(Span::styled(
                item,
                Style::default().fg(Theme::default().foreground),
            )))
        })
        .collect();

    let event_list = List::new(event_items)
        .block(event_block.clone())
        .highlight_symbol(">> ");

    let event_list_scrollbar = Scrollbar::default()
        .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
        .style(Style::default().bg(Theme::default().selection));

    if !app.profile.is_empty()
        && !app.cluster.is_empty()
        && !app.service.is_empty()
        && event_list.is_empty()
        && matches!(app.current_screen, CurrentScreen::Main)
    {
        let loading_block = Paragraph::new("Loading Service Event Logs...")
            .style(Style::default().fg(Theme::default().yellow))
            .block(event_block);
        frame.render_widget(loading_block, chunks[1]);
    } else if event_list.is_empty() {
        let idle_block = Paragraph::new("Configure Data Source to View Logs").block(event_block);
        frame.render_widget(idle_block, chunks[1]);
    } else {
        frame.render_stateful_widget(event_list, chunks[1], &mut app.service_events.state);
        frame.render_stateful_widget(
            event_list_scrollbar,
            chunks[1],
            &mut app.event_box.vertical_scroll_state,
        );
    }

    let current_navigation_text = vec![
        // The first half of the text
        match app.current_screen {
            CurrentScreen::Main => {
                Span::styled("Logging Mode", Style::default().fg(Theme::default().green))
            }
            CurrentScreen::SettingConfig => Span::styled(
                "Set Data Source",
                Style::default().fg(Theme::default().yellow),
            ),
            CurrentScreen::Exiting => {
                Span::styled("Exiting", Style::default().fg(Theme::default().red))
            }
        }
        .to_owned(),
        // A white divider bar to separate the two sections
        Span::styled(" | ", Style::default().fg(Theme::default().foreground)),
        // The final section of the text, with hints on what the user is editing
        {
            if let Some(setting_config) = &app.setting_config {
                match setting_config {
                    SettingConfig::Profile => Span::styled(
                        "Setting AWS Profile",
                        Style::default().fg(Theme::default().green),
                    ),
                    SettingConfig::Cluster => Span::styled(
                        "Setting ECS Cluster",
                        Style::default().fg(Theme::default().green),
                    ),
                    SettingConfig::Service => Span::styled(
                        "Setting ECS Service",
                        Style::default().fg(Theme::default().green),
                    ),
                }
            } else {
                Span::styled(
                    "Not Setting Anything",
                    Style::default().fg(Theme::default().comment),
                )
            }
        },
    ];

    let mode_footer = Paragraph::new(Line::from(current_navigation_text))
        .block(Block::default().borders(Borders::ALL));

    let current_keys_hint = {
        match app.current_screen {
            CurrentScreen::Main => Span::styled(
                "(q) to quit / (c) to config data source",
                Style::default().fg(Theme::default().red),
            ),
            CurrentScreen::SettingConfig => Span::styled(
                "(ESC) to cancel/(Tab) to switch boxes/enter to complete",
                Style::default().fg(Theme::default().red),
            ),
            CurrentScreen::Exiting => Span::styled(
                "(q) to quit / (c) to config data source",
                Style::default().fg(Theme::default().red),
            ),
        }
    };

    let key_notes_footer =
        Paragraph::new(Line::from(current_keys_hint)).block(Block::default().borders(Borders::ALL));

    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    frame.render_widget(mode_footer, footer_chunks[0]);
    frame.render_widget(key_notes_footer, footer_chunks[1]);
    if let Some(setting_config) = &app.setting_config {
        let popup_block = Block::default()
            .title("Setting Data Source")
            .borders(Borders::NONE)
            .style(Style::default().bg(Theme::default().selection));

        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(popup_block, area);

        let popup_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ])
            .split(area);
        let mut profile_block = Block::default().title("AWS Profile").borders(Borders::ALL);
        let mut cluster_block = Block::default().title("ECS Cluster").borders(Borders::ALL);
        let mut service_block = Block::default().title("ECS Service").borders(Borders::ALL);

        let active_style = Style::default().fg(Theme::default().green);

        match setting_config {
            SettingConfig::Profile => {
                profile_block = profile_block.style(active_style);
            }
            SettingConfig::Cluster => {
                cluster_block = cluster_block.style(active_style);
            }
            SettingConfig::Service => {
                service_block = service_block.style(active_style);
            }
        }

        let profile_items: Vec<ListItem> = app
            .profiles
            .items
            .iter()
            .map(|item| {
                if app.profile == *item {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().background)
                            .bg(Theme::default().green),
                    )))
                } else if Some(item) == app.profiles.selected() {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().foreground)
                            .bg(Theme::default().current_line),
                    )))
                } else {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().foreground)
                            .bg(Theme::default().selection),
                    )))
                }
            })
            .collect();

        let profile_list = List::new(profile_items)
            .block(profile_block)
            .highlight_symbol(">> ");

        let profile_list_scrollbar = Scrollbar::default()
            .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .style(Style::default().bg(Theme::default().selection));

        frame.render_stateful_widget(profile_list, popup_chunks[0], &mut app.profiles.state);
        frame.render_stateful_widget(
            profile_list_scrollbar,
            popup_chunks[0],
            &mut app.profile_box.vertical_scroll_state,
        );

        let cluster_items: Vec<ListItem> = app
            .clusters
            .items
            .iter()
            .map(|item| {
                if app.cluster == *item {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().background)
                            .bg(Theme::default().green),
                    )))
                } else if Some(item) == app.clusters.selected() {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().foreground)
                            .bg(Theme::default().current_line),
                    )))
                } else {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().foreground)
                            .bg(Theme::default().selection),
                    )))
                }
            })
            .collect();

        let cluster_list_scrollbar = Scrollbar::default()
            .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .style(Style::default().bg(Theme::default().selection));

        let cluster_list = List::new(cluster_items)
            .block(cluster_block.clone())
            .highlight_symbol(">> ");

        if !app.profile.is_empty() && cluster_list.is_empty() {
            let loading_block = Paragraph::new("Loading Clusters...")
                .style(
                    Style::default()
                        .bg(Theme::default().selection)
                        .fg(Theme::default().yellow),
                )
                .block(cluster_block);
            frame.render_widget(loading_block, popup_chunks[1]);
        } else {
            frame.render_stateful_widget(cluster_list, popup_chunks[1], &mut app.clusters.state);
            frame.render_stateful_widget(
                cluster_list_scrollbar,
                popup_chunks[1],
                &mut app.cluster_box.vertical_scroll_state,
            );
        }

        let service_items: Vec<ListItem> = app
            .services
            .items
            .iter()
            .map(|item| {
                if app.service == *item {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().background)
                            .bg(Theme::default().green),
                    )))
                } else if Some(item) == app.services.selected() {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().foreground)
                            .bg(Theme::default().current_line),
                    )))
                } else {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default()
                            .fg(Theme::default().foreground)
                            .bg(Theme::default().selection),
                    )))
                }
            })
            .collect();

        let service_list = List::new(service_items)
            .block(service_block.clone())
            .highlight_symbol(">> ");
        let service_list_scrollbar = Scrollbar::default()
            .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
            .style(Style::default().bg(Theme::default().selection));

        if !app.cluster.is_empty() && service_list.is_empty() {
            let loading_block = Paragraph::new("Loading Services...")
                .style(
                    Style::default()
                        .bg(Theme::default().selection)
                        .fg(Theme::default().yellow),
                )
                .block(service_block);
            frame.render_widget(loading_block, popup_chunks[2]);
        } else {
            frame.render_stateful_widget(service_list, popup_chunks[2], &mut app.services.state);
            frame.render_stateful_widget(
                service_list_scrollbar,
                popup_chunks[2],
                &mut app.service_box.vertical_scroll_state,
            );
        }
    }
    if let CurrentScreen::Exiting = app.current_screen {
        frame.render_widget(Clear, frame.area()); //this clears the entire screen and anything already drawn
        let popup_block = Block::default()
            .title(" Exit LazyLogger ")
            .padding(Padding::new(2, 2, 2, 2))
            .borders(Borders::ALL)
            .style(Style::default().bg(Theme::default().selection));

        let exit_text = Text::styled(
            "Are you sure you want to exit? (y/n)",
            Style::default().fg(Theme::default().red),
        );
        // the `trim: false` will stop the text from being cut off when over the edge of the block
        let exit_paragraph = Paragraph::new(exit_text)
            .block(popup_block)
            .wrap(Wrap { trim: false });

        let area = centered_rect(60, 25, frame.area());
        frame.render_widget(exit_paragraph, area);
    }
}
