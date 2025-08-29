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
                let previously_selected = app.service_events.state.selected();
                app.service_events.next();

                if app.service_events.state.selected() != previously_selected {
                    app.event_box.vertical_scroll = app.event_box.vertical_scroll.saturating_add(1);
                    app.event_box.vertical_scroll_state = app
                        .event_box
                        .vertical_scroll_state
                        .position(app.event_box.vertical_scroll);
                }
            }
        }
        KeyCode::Up => {
            if app.viewing_logs {
                let previously_selected = app.service_events.state.selected();
                app.service_events.previous();

                if app.service_events.state.selected() != previously_selected {
                    app.event_box.vertical_scroll = app.event_box.vertical_scroll.saturating_sub(1);
                    app.event_box.vertical_scroll_state = app
                        .event_box
                        .vertical_scroll_state
                        .position(app.event_box.vertical_scroll);
                }
            }
        }
        _ => {}
    }
}
