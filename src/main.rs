use aws_config::BehaviorVersion;
use aws_sdk_ecs::{
    Client, Error,
    operation::{
        describe_clusters::DescribeClustersOutput, describe_services::DescribeServicesOutput,
    },
    types::Service,
};
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use itertools::Itertools;
use ratatui::{
    Frame, Terminal,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    layout::{Constraint, Direction, Layout, Rect},
    prelude::{Backend, CrosstermBackend},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Scrollbar,
        ScrollbarState, Wrap,
    },
};
use std::{
    error,
    fs::File,
    io::{self, BufRead},
    time::Duration,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    app.run_app(&mut terminal).await?;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

pub struct OptionList {
    items: Vec<String>,
    state: ListState,
}

impl Default for OptionList {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionList {
    pub fn new() -> Self {
        OptionList {
            items: Vec::new(),
            state: ListState::default(),
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) if i < self.items.len() - 1 => i + 1,
            Some(i) => i,
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) if i > 0 => i - 1,
            Some(i) => i,
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn selected(&self) -> Option<&String> {
        if let Some(i) = self.state.selected() {
            self.items.get(i)
        } else {
            None
        }
    }
}

impl FromIterator<String> for OptionList {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let items: Vec<String> = iter.into_iter().collect();
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self { items, state }
    }
}

pub enum CurrentScreen {
    Main,
    SettingConfig,
    Exiting,
}

pub struct ProfileBox {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

pub struct ClusterBox {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

pub struct ServiceBox {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

pub struct EventLogBox {
    pub vertical_scroll_state: ScrollbarState,
    pub vertical_scroll: usize,
}

pub enum SettingConfig {
    ProfileBox,
    ClusterBox,
    ServiceBox,
}

pub struct Theme {
    pub background: Color,
    pub current_line: Color,
    pub selection: Color,
    pub foreground: Color,
    pub comment: Color,
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub cyan: Color,
    pub purple: Color,
    pub pink: Color,
}

impl Default for Theme {
    fn default() -> Self {
        //Dracula Theme
        Theme {
            background: Color::Rgb(40, 42, 54),
            current_line: Color::Rgb(98, 114, 164),
            selection: Color::Rgb(68, 71, 90),
            foreground: Color::Rgb(248, 248, 242),
            comment: Color::Rgb(98, 114, 164),
            red: Color::Rgb(255, 85, 85),
            orange: Color::Rgb(255, 184, 108),
            yellow: Color::Rgb(241, 250, 140),
            green: Color::Rgb(80, 250, 123),
            cyan: Color::Rgb(139, 233, 253),
            purple: Color::Rgb(189, 147, 249),
            pink: Color::Rgb(255, 121, 198),
        }
    }
}

pub struct App {
    pub exit: bool,
    pub profile: String,
    pub profiles: OptionList,
    pub cluster: String,
    pub clusters: OptionList,
    pub service: String,
    pub services: OptionList,
    pub service_events: OptionList,
    pub current_screen: CurrentScreen,
    pub setting_config: Option<SettingConfig>,
    pub profile_box: ProfileBox,
    pub cluster_box: ClusterBox,
    pub service_box: ServiceBox,
    pub event_box: EventLogBox,
    pub viewing_logs: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> App {
        App {
            exit: false,
            profile: String::new(),
            profiles: OptionList::new(),
            cluster: String::new(),
            clusters: OptionList::new(),
            service: String::new(),
            services: OptionList::new(),
            service_events: OptionList::new(),
            current_screen: CurrentScreen::Main,
            setting_config: None,
            profile_box: ProfileBox {
                vertical_scroll_state: ScrollbarState::default(),
                vertical_scroll: 0,
            },
            cluster_box: ClusterBox {
                vertical_scroll_state: ScrollbarState::default(),
                vertical_scroll: 0,
            },
            service_box: ServiceBox {
                vertical_scroll_state: ScrollbarState::default(),
                vertical_scroll: 0,
            },
            event_box: EventLogBox {
                vertical_scroll_state: ScrollbarState::default(),
                vertical_scroll: 0,
            },
            viewing_logs: false,
        }
    }

    async fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<bool> {
        let tick_rate = Duration::from_millis(250);
        let mut last_tick = std::time::Instant::now();
        let mut dirty = true;
        loop {
            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == event::KeyEventKind::Release {
                        // Skip events that are not KeyEventKind::Press
                        continue;
                    }
                    match self.current_screen {
                        CurrentScreen::Main => match key.code {
                            KeyCode::Char('c') => {
                                self.current_screen = CurrentScreen::SettingConfig;
                                self.setting_config = Some(SettingConfig::ProfileBox);
                            }
                            KeyCode::Char('q') => {
                                self.current_screen = CurrentScreen::Exiting;
                            }
                            KeyCode::Char('e') => {
                                self.viewing_logs = !self.viewing_logs;
                            }
                            KeyCode::Char('r') => {
                                if self.viewing_logs {
                                    self.service_events = OptionList::new();
                                }
                            }
                            KeyCode::Down => {
                                if self.viewing_logs {
                                    let previously_selected = self.service_events.state.selected();
                                    self.service_events.next();

                                    if self.service_events.state.selected() != previously_selected {
                                        self.event_box.vertical_scroll =
                                            self.event_box.vertical_scroll.saturating_add(1);
                                        self.event_box.vertical_scroll_state = self
                                            .event_box
                                            .vertical_scroll_state
                                            .position(self.event_box.vertical_scroll);
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if self.viewing_logs {
                                    let previously_selected = self.service_events.state.selected();
                                    self.service_events.previous();

                                    if self.service_events.state.selected() != previously_selected {
                                        self.event_box.vertical_scroll =
                                            self.event_box.vertical_scroll.saturating_sub(1);
                                        self.event_box.vertical_scroll_state = self
                                            .event_box
                                            .vertical_scroll_state
                                            .position(self.event_box.vertical_scroll);
                                    }
                                }
                            }
                            _ => {}
                        },
                        CurrentScreen::Exiting => match key.code {
                            KeyCode::Char('y') => {
                                return Ok(true);
                            }
                            KeyCode::Char('n') | KeyCode::Char('q') => {
                                self.current_screen = CurrentScreen::Main;
                            }
                            _ => {}
                        },
                        CurrentScreen::SettingConfig => match key.code {
                            KeyCode::Esc => {
                                self.current_screen = CurrentScreen::Main;
                                self.setting_config = None;
                            }
                            KeyCode::Tab => {
                                self.toggle_setting();
                            }
                            KeyCode::Char('q') => {
                                self.current_screen = CurrentScreen::Main;
                                self.setting_config = None;
                            }
                            KeyCode::Enter => {
                                if let Some(setting_config) = &self.setting_config {
                                    match setting_config {
                                        SettingConfig::ProfileBox => {
                                            if self.profiles.selected().is_some() {
                                                self.profile =
                                                    self.profiles.selected().unwrap().to_string();
                                                self.setting_config =
                                                    Some(SettingConfig::ClusterBox);
                                                self.clusters = OptionList::new();
                                                self.services = OptionList::new();
                                                self.cluster.clear();
                                                self.service.clear();
                                                self.service_events = OptionList::new();
                                            }
                                        }
                                        SettingConfig::ClusterBox => {
                                            if self.clusters.selected().is_some() {
                                                self.cluster =
                                                    self.clusters.selected().unwrap().to_string();
                                                self.setting_config =
                                                    Some(SettingConfig::ServiceBox);
                                                self.services = OptionList::new();
                                                self.service.clear();
                                            }
                                        }
                                        SettingConfig::ServiceBox => {
                                            if self.services.selected().is_some() {
                                                self.service =
                                                    self.services.selected().unwrap().to_string();
                                                self.current_screen = CurrentScreen::Main;
                                                self.setting_config = None;
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if let Some(setting_config) = &self.setting_config {
                                    match setting_config {
                                        SettingConfig::ProfileBox => {
                                            let previously_selected =
                                                self.profiles.state.selected();
                                            self.profiles.next();

                                            if self.profiles.state.selected() != previously_selected
                                            {
                                                self.profile_box.vertical_scroll = self
                                                    .profile_box
                                                    .vertical_scroll
                                                    .saturating_add(1);
                                                self.profile_box.vertical_scroll_state = self
                                                    .profile_box
                                                    .vertical_scroll_state
                                                    .position(self.profile_box.vertical_scroll);
                                            }
                                        }
                                        SettingConfig::ClusterBox => {
                                            let previously_selected =
                                                self.clusters.state.selected();
                                            self.clusters.next();
                                            if self.clusters.state.selected() != previously_selected
                                            {
                                                self.cluster_box.vertical_scroll = self
                                                    .cluster_box
                                                    .vertical_scroll
                                                    .saturating_add(1);
                                                self.cluster_box.vertical_scroll_state = self
                                                    .cluster_box
                                                    .vertical_scroll_state
                                                    .position(self.cluster_box.vertical_scroll);
                                            }
                                        }
                                        SettingConfig::ServiceBox => {
                                            let previously_selected =
                                                self.services.state.selected();
                                            self.services.next();
                                            if self.services.state.selected() != previously_selected
                                            {
                                                self.service_box.vertical_scroll = self
                                                    .service_box
                                                    .vertical_scroll
                                                    .saturating_add(1);
                                                self.service_box.vertical_scroll_state = self
                                                    .service_box
                                                    .vertical_scroll_state
                                                    .position(self.service_box.vertical_scroll);
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if let Some(setting_config) = &self.setting_config {
                                    match setting_config {
                                        SettingConfig::ProfileBox => {
                                            let previously_selected =
                                                self.profiles.state.selected();
                                            self.profiles.previous();
                                            if self.profiles.state.selected() != previously_selected
                                            {
                                                self.profile_box.vertical_scroll = self
                                                    .profile_box
                                                    .vertical_scroll
                                                    .saturating_sub(1);
                                                self.profile_box.vertical_scroll_state = self
                                                    .profile_box
                                                    .vertical_scroll_state
                                                    .position(self.profile_box.vertical_scroll);
                                            }
                                        }
                                        SettingConfig::ClusterBox => {
                                            let previously_selected =
                                                self.clusters.state.selected();
                                            self.clusters.previous();
                                            if self.clusters.state.selected() != previously_selected
                                            {
                                                self.cluster_box.vertical_scroll = self
                                                    .cluster_box
                                                    .vertical_scroll
                                                    .saturating_sub(1);
                                                self.cluster_box.vertical_scroll_state = self
                                                    .cluster_box
                                                    .vertical_scroll_state
                                                    .position(self.cluster_box.vertical_scroll);
                                            }
                                        }
                                        SettingConfig::ServiceBox => {
                                            let previously_selected =
                                                self.services.state.selected();
                                            self.services.previous();
                                            if self.services.state.selected() != previously_selected
                                            {
                                                self.service_box.vertical_scroll = self
                                                    .service_box
                                                    .vertical_scroll
                                                    .saturating_sub(1);
                                                self.service_box.vertical_scroll_state = self
                                                    .service_box
                                                    .vertical_scroll_state
                                                    .position(self.service_box.vertical_scroll);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        },
                    }
                    dirty = true;
                }
            }

            if last_tick.elapsed() >= tick_rate {
                self.on_tick().await;
                last_tick = std::time::Instant::now();
                dirty = true;
            }

            if dirty {
                terminal.draw(|f| ui(f, self))?;
                dirty = false;
            }
        }
    }

    async fn on_tick(&mut self) {
        if let CurrentScreen::Main = &self.current_screen {
            if !self.profile.is_empty()
                && !self.cluster.is_empty()
                && !self.service.is_empty()
                && self.service_events.items.is_empty()
            {
                let aws_config = aws_config::defaults(BehaviorVersion::latest())
                    .region("us-east-1")
                    .profile_name(&self.profile)
                    .load()
                    .await;
                let client = Client::new(&aws_config);
                if let Ok(service) = get_services(&client, &self.cluster).await {
                    if let Some(services) = service.services {
                        if let Some(service_obj) = services
                            .iter()
                            .find(|s| s.service_name().unwrap_or_default() == self.service)
                        {
                            if let Ok(events) = get_events(service_obj).await {
                                self.service_events = OptionList::from_iter(events);
                                self.event_box.vertical_scroll_state = self
                                    .event_box
                                    .vertical_scroll_state
                                    .content_length(self.service_events.items.len());
                            }
                        }
                    }
                }
            }
        }
        if let CurrentScreen::SettingConfig = &self.current_screen {
            if !self.service_events.items.is_empty() {
                self.service_events = OptionList::new();
            }
        }
        if let Some(setting_config) = &self.setting_config {
            match setting_config {
                SettingConfig::ProfileBox => {
                    if self.profiles.items.is_empty() {
                        // Load profiles if not already loaded
                        let profiles = get_profiles().await.unwrap();
                        self.profiles = OptionList::from_iter(profiles);
                        self.profile_box.vertical_scroll_state = self
                            .profile_box
                            .vertical_scroll_state
                            .content_length(self.profiles.items.len());
                    }
                }
                SettingConfig::ClusterBox => {
                    if !self.profile.is_empty() && self.clusters.items.is_empty() {
                        let aws_config = aws_config::defaults(BehaviorVersion::latest())
                            .region("us-east-1")
                            .profile_name(self.profiles.selected().unwrap())
                            .load()
                            .await;
                        let client = Client::new(&aws_config);
                        let cluster = get_clusters(&client).await.unwrap();
                        self.clusters = OptionList::from_iter(
                            cluster
                                .clusters
                                .unwrap()
                                .iter()
                                .map(|c| c.cluster_name().unwrap().to_string())
                                .collect::<Vec<String>>(),
                        );
                        self.cluster_box.vertical_scroll_state = self
                            .cluster_box
                            .vertical_scroll_state
                            .content_length(self.clusters.items.len());
                    }
                }
                SettingConfig::ServiceBox => {
                    if !self.profile.is_empty()
                        && !self.cluster.is_empty()
                        && self.services.items.is_empty()
                    {
                        let aws_config = aws_config::defaults(BehaviorVersion::latest())
                            .region("us-east-1")
                            .profile_name(self.profiles.selected().unwrap())
                            .load()
                            .await;
                        let client = Client::new(&aws_config);
                        let service = get_services(&client, &self.cluster).await.unwrap();
                        self.services = OptionList::from_iter(
                            service
                                .services
                                .unwrap()
                                .iter()
                                .map(|s| s.service_name().unwrap().to_string())
                                .collect::<Vec<String>>(),
                        );
                        self.service_box.vertical_scroll_state = self
                            .service_box
                            .vertical_scroll_state
                            .content_length(self.services.items.len());
                    }
                }
            }
        }
    }

    pub fn toggle_setting(&mut self) {
        if let Some(config_mode) = &self.setting_config {
            match config_mode {
                SettingConfig::ProfileBox => {
                    self.setting_config = Some(SettingConfig::ClusterBox);
                }
                SettingConfig::ClusterBox => {
                    self.setting_config = Some(SettingConfig::ServiceBox);
                }
                SettingConfig::ServiceBox => {
                    self.setting_config = Some(SettingConfig::ProfileBox);
                }
            }
        } else {
            self.setting_config = Some(SettingConfig::ProfileBox);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
                    SettingConfig::ProfileBox => Span::styled(
                        "Setting AWS Profile",
                        Style::default().fg(Theme::default().green),
                    ),
                    SettingConfig::ClusterBox => Span::styled(
                        "Setting ECS Cluster",
                        Style::default().fg(Theme::default().green),
                    ),
                    SettingConfig::ServiceBox => Span::styled(
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
            SettingConfig::ProfileBox => {
                profile_block = profile_block.style(active_style);
            }
            SettingConfig::ClusterBox => {
                cluster_block = cluster_block.style(active_style);
            }
            SettingConfig::ServiceBox => {
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
//========================================================================
async fn get_profiles() -> Result<Vec<String>, Box<dyn error::Error>> {
    let mut profiles = Vec::new();
    let cred_file = File::open(dirs::home_dir().unwrap().join(".aws").join("credentials")).unwrap();
    let read_creds = io::BufReader::new(cred_file);
    for line in read_creds.lines() {
        let line = line.unwrap();
        if line.starts_with('[') && line.ends_with(']') {
            let profile = line.trim_matches(&['[', ']'][..]);
            profiles.push(profile.to_string());
        }
    }
    profiles.sort();
    Ok(profiles)
}

async fn get_clusters(client: &Client) -> Result<DescribeClustersOutput, Error> {
    let resp = client.list_clusters().send().await?;
    let mut cluster_arns = resp.cluster_arns().to_vec();
    cluster_arns.sort();
    let cluster = client
        .describe_clusters()
        .set_clusters(Some(cluster_arns))
        .send()
        .await?;
    Ok(cluster)
}

async fn get_services(
    client: &Client,
    cluster_name: &str,
) -> Result<DescribeServicesOutput, Error> {
    let mut next_token = None;
    let mut service_arns: Vec<String> = Vec::new();

    loop {
        let resp = client
            .list_services()
            .cluster(cluster_name)
            .set_next_token(next_token.clone())
            .send()
            .await?;

        service_arns.extend(resp.service_arns().to_vec());

        if let Some(token) = resp.next_token() {
            next_token = Some(token.to_string());
        } else {
            break;
        }
    }

    service_arns.sort();
    let mut all_services: Vec<_> = Vec::new();

    for chunk in &service_arns.into_iter().chunks(10) {
        let resp = client
            .describe_services()
            .cluster(cluster_name)
            .set_services(Some(chunk.collect()))
            .send()
            .await?;
        if let Some(s) = resp.services {
            all_services.extend(s);
        }
    }

    let output = DescribeServicesOutput::builder()
        .set_services(Some(all_services))
        .build();

    Ok(output)
}

/*
async fn get_tasks(
    client: &Client,
    cluster_name: &str,
    service_name: &str,
) -> Result<DescribeTasksOutput, Error> {
    let resp = client
        .list_tasks()
        .cluster(cluster_name)
        .service_name(service_name)
        .send()
        .await?;
    let task_arns = resp.task_arns();
    let tasks = client
        .describe_tasks()
        .cluster(cluster_name)
        .set_tasks(Some(task_arns.into()))
        .send()
        .await?;
    Ok(tasks)
}
*/

async fn get_events(service: &Service) -> Result<Vec<String>, Error> {
    let logs = service.events();
    let mut formatted_logs = Vec::new();
    for entry in logs {
        formatted_logs.push(format!(
            "[{}] {}",
            entry.created_at().unwrap(),
            entry.message().unwrap(),
        ));
    }
    Ok(formatted_logs)
}
