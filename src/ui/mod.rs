use crate::app::{App, Overlay, Page};
use ratatui::Frame;

pub mod home;
pub mod loading;
pub mod login;
pub mod author;
pub mod page_lyrics;
pub mod player_bar;
pub mod playlist;
pub mod search;
pub mod search_box;
pub mod settings;
pub mod theme;

pub fn draw(frame: &mut Frame, app: &mut App) {
	match app.page {
		Page::Login => login::draw_login(frame, app),
		Page::Loading => loading::draw_loading(frame, app),
		Page::Home => home::draw_home(frame, app),
		Page::Playlist => playlist::draw_playlist(frame, app),
		Page::Author => author::draw_author(frame, app),
		Page::Search => search::draw_search(frame, app),
	}

	if matches!(
		app.overlay,
		Some(Overlay::Settings)
			| Some(Overlay::SettingsPlayback)
			| Some(Overlay::SettingsKeybinds)
			| Some(Overlay::SettingsAbout)
	) {
		settings::draw_settings_modal(frame, app);
	}
	if matches!(app.overlay, Some(Overlay::SearchBox)) {
		search_box::draw_search_box_overlay(frame, app);
	}
}
