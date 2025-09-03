use crate::keymaps::{
    exit_screen_keymaps, log_details_keymaps, main_screen_keymaps, setting_config_keymaps,
};
use crate::ui::ui;

use crate::aws_utils::{get_clusters, get_log_group_name, get_logs, get_profiles, get_services};
use aws_config::BehaviorVersion;
use aws_sdk_ecs::Client;
use crossterm::event::{self, Event};
use ratatui::{
    Terminal,
    prelude::Backend,
    style::Color,
    widgets::{ListState, ScrollbarState},
};
use std::{
    io::{self},
    time::Duration,
};

pub struct OptionList {
    pub items: Vec<String>,
    pub state: ListState,
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
    LogDetails,
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
    Profile,
    Cluster,
    Service,
}

pub struct Theme {
    pub background: Color,
    pub current_line: Color,
    pub selection: Color,
    pub foreground: Color,
    pub comment: Color,
    pub red: Color,
    // pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    // pub cyan: Color,
    // pub purple: Color,
    // pub pink: Color,
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
            // orange: Color::Rgb(255, 184, 108),
            yellow: Color::Rgb(241, 250, 140),
            green: Color::Rgb(80, 250, 123),
            // cyan: Color::Rgb(139, 233, 253),
            // purple: Color::Rgb(189, 147, 249),
            // pink: Color::Rgb(255, 121, 198),
        }
    }
}

pub struct App {
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

    pub async fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<bool> {
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
                        CurrentScreen::Main => {
                            main_screen_keymaps(key, self);
                        }
                        CurrentScreen::Exiting => {
                            if exit_screen_keymaps(key, self)? {
                                return Ok(true);
                            }
                        }
                        CurrentScreen::LogDetails => {
                            log_details_keymaps(key, self);
                        }
                        CurrentScreen::SettingConfig => {
                            setting_config_keymaps(key, self);
                        }
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
                let ecs_client = aws_sdk_ecs::Client::new(&aws_config);
                let cw_client = aws_sdk_cloudwatchlogs::Client::new(&aws_config);
                if let Ok(service) = get_services(&ecs_client, &self.cluster).await {
                    if let Some(services) = service.services {
                        if let Some(service_obj) = services
                            .iter()
                            .find(|s| s.service_name().unwrap_or_default() == self.service)
                        {
                            let log_group = get_log_group_name(&ecs_client, service_obj).await;
                            if let Ok(events) = get_logs(&cw_client, &log_group.unwrap()).await {
                                self.service_events = OptionList::from_iter(events);
                                self.event_box.vertical_scroll_state = self
                                    .event_box
                                    .vertical_scroll_state
                                    .content_length(self.service_events.items.len())
                                    .position(self.service_events.items.len());
                                self.service_events.state.select(Some(
                                    self.service_events.items.len().saturating_sub(1),
                                ));
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
                SettingConfig::Profile => {
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
                        self.cluster_box.vertical_scroll_state = self
                            .cluster_box
                            .vertical_scroll_state
                            .content_length(self.clusters.items.len());
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
