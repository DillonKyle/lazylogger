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
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
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
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
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

pub enum SettingConfig {
    Profile,
    Cluster,
    Service,
}

pub struct App {
    pub exit: bool,
    pub profile: String,
    pub profiles: OptionList,
    pub cluster: String,
    pub clusters: OptionList,
    pub service: String,
    pub services: OptionList,
    pub service_events: Vec<String>,
    pub current_screen: CurrentScreen,
    pub setting_config: Option<SettingConfig>,
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
            service_events: Vec::new(),
            current_screen: CurrentScreen::Main,
            setting_config: None,
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
                                self.setting_config = Some(SettingConfig::Profile);
                            }
                            KeyCode::Char('q') => {
                                self.current_screen = CurrentScreen::Exiting;
                            }
                            _ => {}
                        },
                        CurrentScreen::Exiting => match key.code {
                            KeyCode::Char('y') => {
                                return Ok(true);
                            }
                            KeyCode::Char('n') | KeyCode::Char('q') => {
                                return Ok(false);
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
                                        SettingConfig::Profile => {
                                            if self.profiles.selected().is_some() {
                                                self.profile =
                                                    self.profiles.selected().unwrap().to_string();
                                                self.setting_config = Some(SettingConfig::Cluster);
                                                self.clusters = OptionList::new();
                                                self.services = OptionList::new();
                                                self.cluster.clear();
                                                self.service.clear();
                                                self.service_events.clear();
                                            }
                                        }
                                        SettingConfig::Cluster => {
                                            if self.clusters.selected().is_some() {
                                                self.cluster =
                                                    self.clusters.selected().unwrap().to_string();
                                                self.setting_config = Some(SettingConfig::Service);
                                                self.services = OptionList::new();
                                                self.service.clear();
                                            }
                                        }
                                        SettingConfig::Service => {
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
                                        SettingConfig::Profile => {
                                            self.profiles.next();
                                        }
                                        SettingConfig::Cluster => {
                                            self.clusters.next();
                                        }
                                        SettingConfig::Service => {
                                            self.services.next();
                                        }
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if let Some(setting_config) = &self.setting_config {
                                    match setting_config {
                                        SettingConfig::Profile => {
                                            self.profiles.previous();
                                        }
                                        SettingConfig::Cluster => {
                                            self.clusters.previous();
                                        }
                                        SettingConfig::Service => {
                                            self.services.previous();
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
            if !self.profile.is_empty() && !self.cluster.is_empty() && !self.service.is_empty() {
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
                                self.service_events = events;
                            }
                        }
                    }
                }
            }
        }
        if let CurrentScreen::SettingConfig = &self.current_screen {
            if !self.service_events.is_empty() {
                self.service_events.clear();
            }
        }
        if let Some(setting_config) = &self.setting_config {
            match setting_config {
                SettingConfig::Profile => {
                    if self.profiles.items.is_empty() {
                        // Load profiles if not already loaded
                        let profiles = get_profiles().await.unwrap();
                        self.profiles = OptionList::from_iter(profiles);
                    }
                }
                SettingConfig::Cluster => {
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
                    }
                }
                SettingConfig::Service => {
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
                    }
                }
            }
        }
    }

    pub fn toggle_setting(&mut self) {
        if let Some(config_mode) = &self.setting_config {
            match config_mode {
                SettingConfig::Profile => {
                    self.setting_config = Some(SettingConfig::Cluster);
                }
                SettingConfig::Cluster => {
                    self.setting_config = Some(SettingConfig::Service);
                }
                SettingConfig::Service => {
                    self.setting_config = Some(SettingConfig::Profile);
                }
            }
        } else {
            self.setting_config = Some(SettingConfig::Profile);
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

pub fn ui(frame: &mut Frame, app: &App) {
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
        Style::default().fg(Color::Green),
    ))
    .block(title_block);

    frame.render_widget(title, chunks[0]);

    let event_block = Block::default()
        .title("Service Events")
        .borders(Borders::ALL)
        .style(Style::default());

    let event_items: Vec<ListItem> = app
        .service_events
        .iter()
        .map(|item| {
            ListItem::new(Line::from(Span::styled(
                item,
                Style::default().fg(Color::White),
            )))
        })
        .collect();

    let event_list = List::new(event_items).block(event_block);

    frame.render_widget(event_list, chunks[1]);

    let current_navigation_text = vec![
        // The first half of the text
        match app.current_screen {
            CurrentScreen::Main => Span::styled("Normal Mode", Style::default().fg(Color::Green)),
            CurrentScreen::SettingConfig => {
                Span::styled("Set Config", Style::default().fg(Color::Yellow))
            }
            CurrentScreen::Exiting => Span::styled("Exiting", Style::default().fg(Color::LightRed)),
        }
        .to_owned(),
        // A white divider bar to separate the two sections
        Span::styled(" | ", Style::default().fg(Color::White)),
        // The final section of the text, with hints on what the user is editing
        {
            if let Some(setting_config) = &app.setting_config {
                match setting_config {
                    SettingConfig::Profile => {
                        Span::styled("Setting AWS Profile", Style::default().fg(Color::Green))
                    }
                    SettingConfig::Cluster => {
                        Span::styled("Setting ECS Cluster", Style::default().fg(Color::Green))
                    }
                    SettingConfig::Service => {
                        Span::styled("Setting ECS Service", Style::default().fg(Color::Green))
                    }
                }
            } else {
                Span::styled("Not Setting Anything", Style::default().fg(Color::DarkGray))
            }
        },
    ];

    let mode_footer = Paragraph::new(Line::from(current_navigation_text))
        .block(Block::default().borders(Borders::ALL));

    let current_keys_hint = {
        match app.current_screen {
            CurrentScreen::Main => Span::styled(
                "(q) to quit / (c) to config",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::SettingConfig => Span::styled(
                "(ESC) to cancel/(Tab) to switch boxes/enter to complete",
                Style::default().fg(Color::Red),
            ),
            CurrentScreen::Exiting => Span::styled(
                "(q) to quit / (c) to config",
                Style::default().fg(Color::Red),
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
            .title("Setting Configuration")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));

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

        let active_style = Style::default().fg(Color::Green);

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
                        Style::default().fg(Color::Black).bg(Color::Green),
                    )))
                } else if Some(item) == app.profiles.selected() {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black).bg(Color::LightYellow),
                    )))
                } else {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black),
                    )))
                }
            })
            .collect();

        let profile_list = List::new(profile_items)
            .block(profile_block)
            .highlight_symbol(">> ");

        frame.render_widget(profile_list, popup_chunks[0]);

        let cluster_items: Vec<ListItem> = app
            .clusters
            .items
            .iter()
            .map(|item| {
                if app.cluster == *item {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black).bg(Color::Green),
                    )))
                } else if Some(item) == app.clusters.selected() {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black).bg(Color::LightYellow),
                    )))
                } else {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black),
                    )))
                }
            })
            .collect();

        let cluster_list = List::new(cluster_items)
            .block(cluster_block)
            .highlight_symbol(">> ");

        frame.render_widget(cluster_list, popup_chunks[1]);

        let service_items: Vec<ListItem> = app
            .services
            .items
            .iter()
            .map(|item| {
                if app.service == *item {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black).bg(Color::Green),
                    )))
                } else if Some(item) == app.services.selected() {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black).bg(Color::LightYellow),
                    )))
                } else {
                    ListItem::new(Line::from(Span::styled(
                        item,
                        Style::default().fg(Color::Black),
                    )))
                }
            })
            .collect();

        let service_list = List::new(service_items)
            .block(service_block)
            .highlight_symbol(">> ");

        frame.render_widget(service_list, popup_chunks[2]);
    }
    if let CurrentScreen::Exiting = app.current_screen {
        frame.render_widget(Clear, frame.area()); //this clears the entire screen and anything already drawn
        let popup_block = Block::default()
            .title("Exit LazyLogger")
            .borders(Borders::NONE)
            .style(Style::default().bg(Color::DarkGray));

        let exit_text = Text::styled(
            "Are you sure you want to exit? (y/n)",
            Style::default().fg(Color::Red),
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
    Ok(profiles)
}

async fn get_clusters(client: &Client) -> Result<DescribeClustersOutput, Error> {
    let resp = client.list_clusters().send().await?;
    let cluster_arns = resp.cluster_arns();
    let cluster = client
        .describe_clusters()
        .set_clusters(Some(cluster_arns.into()))
        .send()
        .await?;
    Ok(cluster)
}

async fn get_services(
    client: &Client,
    cluster_name: &str,
) -> Result<DescribeServicesOutput, Error> {
    let resp = client.list_services().cluster(cluster_name).send().await?;
    let service_arns = resp.service_arns();
    let services = client
        .describe_services()
        .cluster(cluster_name)
        .set_services(Some(service_arns.into()))
        .send()
        .await?;
    Ok(services)
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
