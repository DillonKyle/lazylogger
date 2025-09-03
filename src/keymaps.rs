use crate::app::{App, CurrentScreen, OptionList, SettingConfig};
use crossterm::event::{KeyCode, KeyEvent};

pub fn main_screen_keymaps(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('c') => {
            app.current_screen = CurrentScreen::SettingConfig;
            app.setting_config = Some(SettingConfig::Profile);
        }
        KeyCode::Char('q') => {
            app.current_screen = CurrentScreen::Exiting;
        }
        KeyCode::Char('e') => {
            app.viewing_logs = !app.viewing_logs;
        }
        KeyCode::Char('r') => {
            if app.viewing_logs {
                app.service_events = OptionList::new();
            }
        }
        KeyCode::Down => {
            if app.viewing_logs {
                app.service_events.next();
            }
        }
        KeyCode::Up => {
            if app.viewing_logs {
                app.service_events.previous();
            }
        }
        KeyCode::Enter => {
            if app.viewing_logs {
                app.current_screen = CurrentScreen::LogDetails;
            }
        }
        _ => {}
    }
}

pub fn exit_screen_keymaps(key: KeyEvent, app: &mut App) -> std::io::Result<bool> {
    match key.code {
        KeyCode::Char('y') => Ok(true),
        KeyCode::Char('n') | KeyCode::Char('q') => {
            app.current_screen = CurrentScreen::Main;
            Ok(false)
        }
        _ => Ok(false),
    }
}

pub fn log_details_keymaps(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.current_screen = CurrentScreen::Main;
        }
        _ => {}
    }
}

pub fn setting_config_keymaps(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Esc => {
            app.current_screen = CurrentScreen::Main;
            app.setting_config = None;
        }
        KeyCode::Tab => {
            app.toggle_setting();
        }
        KeyCode::Char('q') => {
            app.current_screen = CurrentScreen::Main;
            app.setting_config = None;
        }
        KeyCode::Enter => {
            if let Some(setting_config) = &app.setting_config {
                match setting_config {
                    SettingConfig::Profile => {
                        if app.profiles.selected().is_some() {
                            app.profile = app.profiles.selected().unwrap().to_string();
                            app.setting_config = Some(SettingConfig::Cluster);
                            app.clusters = OptionList::new();
                            app.services = OptionList::new();
                            app.cluster.clear();
                            app.service.clear();
                            app.service_events = OptionList::new();
                        }
                    }
                    SettingConfig::Cluster => {
                        if app.clusters.selected().is_some() {
                            app.cluster = app.clusters.selected().unwrap().to_string();
                            app.setting_config = Some(SettingConfig::Service);
                            app.services = OptionList::new();
                            app.service.clear();
                        }
                    }
                    SettingConfig::Service => {
                        if app.services.selected().is_some() {
                            app.service = app.services.selected().unwrap().to_string();
                            app.current_screen = CurrentScreen::Main;
                            app.setting_config = None;
                        }
                    }
                }
            }
        }
        KeyCode::Down => {
            if let Some(setting_config) = &app.setting_config {
                match setting_config {
                    SettingConfig::Profile => {
                        let previously_selected = app.profiles.state.selected();
                        app.profiles.next();

                        if app.profiles.state.selected() != previously_selected {
                            app.profile_box.vertical_scroll =
                                app.profile_box.vertical_scroll.saturating_add(1);
                            app.profile_box.vertical_scroll_state = app
                                .profile_box
                                .vertical_scroll_state
                                .position(app.profile_box.vertical_scroll);
                        }
                    }
                    SettingConfig::Cluster => {
                        let previously_selected = app.clusters.state.selected();
                        app.clusters.next();
                        if app.clusters.state.selected() != previously_selected {
                            app.cluster_box.vertical_scroll =
                                app.cluster_box.vertical_scroll.saturating_add(1);
                            app.cluster_box.vertical_scroll_state = app
                                .cluster_box
                                .vertical_scroll_state
                                .position(app.cluster_box.vertical_scroll);
                        }
                    }
                    SettingConfig::Service => {
                        let previously_selected = app.services.state.selected();
                        app.services.next();
                        if app.services.state.selected() != previously_selected {
                            app.service_box.vertical_scroll =
                                app.service_box.vertical_scroll.saturating_add(1);
                            app.service_box.vertical_scroll_state = app
                                .service_box
                                .vertical_scroll_state
                                .position(app.service_box.vertical_scroll);
                        }
                    }
                }
            }
        }
        KeyCode::Up => {
            if let Some(setting_config) = &app.setting_config {
                match setting_config {
                    SettingConfig::Profile => {
                        let previously_selected = app.profiles.state.selected();
                        app.profiles.previous();
                        if app.profiles.state.selected() != previously_selected {
                            app.profile_box.vertical_scroll =
                                app.profile_box.vertical_scroll.saturating_sub(1);
                            app.profile_box.vertical_scroll_state = app
                                .profile_box
                                .vertical_scroll_state
                                .position(app.profile_box.vertical_scroll);
                        }
                    }
                    SettingConfig::Cluster => {
                        let previously_selected = app.clusters.state.selected();
                        app.clusters.previous();
                        if app.clusters.state.selected() != previously_selected {
                            app.cluster_box.vertical_scroll =
                                app.cluster_box.vertical_scroll.saturating_sub(1);
                            app.cluster_box.vertical_scroll_state = app
                                .cluster_box
                                .vertical_scroll_state
                                .position(app.cluster_box.vertical_scroll);
                        }
                    }
                    SettingConfig::Service => {
                        let previously_selected = app.services.state.selected();
                        app.services.previous();
                        if app.services.state.selected() != previously_selected {
                            app.service_box.vertical_scroll =
                                app.service_box.vertical_scroll.saturating_sub(1);
                            app.service_box.vertical_scroll_state = app
                                .service_box
                                .vertical_scroll_state
                                .position(app.service_box.vertical_scroll);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
