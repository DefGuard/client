use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::{query, FromRow, Type};
use struct_patch::Patch;
use strum::{AsRefStr, EnumString};
use tracing::Level;

use crate::{database::DbPool, error::Error};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type, EnumString, AsRefStr)]
#[sqlx(type_name = "theme", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SettingsTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type, EnumString)]
#[sqlx(type_name = "log_level", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SettingsLogLevel {
    Error,
    Info,
    Debug,
    Trace,
}

impl From<SettingsLogLevel> for Level {
    fn from(val: SettingsLogLevel) -> Self {
        match val {
            SettingsLogLevel::Error => Self::ERROR,
            SettingsLogLevel::Info => Self::INFO,
            SettingsLogLevel::Debug => Self::DEBUG,
            SettingsLogLevel::Trace => Self::TRACE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type, EnumString, AsRefStr)]
#[sqlx(type_name = "tray_icon_theme", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TrayIconTheme {
    Color,
    White,
    Black,
    Gray,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type, EnumString, AsRefStr)]
#[sqlx(type_name = "selected_view", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ClientView {
    Grid = 0,
    Detail,
}

#[derive(FromRow, Debug, Serialize, Deserialize, Patch)]
#[patch_derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(skip)]
    pub id: Option<i64>,
    pub theme: SettingsTheme,
    pub log_level: SettingsLogLevel,
    pub tray_icon_theme: TrayIconTheme,
    pub check_for_updates: bool,
    pub selected_view: Option<ClientView>,
}

impl Settings {
    pub async fn get(pool: &DbPool) -> Result<Self, Error> {
        let query_res = query!("SELECT * FROM settings WHERE id = 1;")
            .fetch_one(pool)
            .await?;
        let settings = Self {
            id: Some(query_res.id),
            log_level: SettingsLogLevel::from_str(&query_res.log_level)?,
            theme: SettingsTheme::from_str(&query_res.theme)?,
            tray_icon_theme: TrayIconTheme::from_str(&query_res.tray_icon_theme)?,
            check_for_updates: query_res.check_for_updates,
            selected_view: match &query_res.selected_view {
                Some(selected_view) => Some(ClientView::from_str(selected_view)?),
                None => None,
            },
        };
        Ok(settings)
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        query!(
            "UPDATE settings \
            SET theme = $1, log_level = $2, tray_icon_theme = $3, check_for_updates = $4, selected_view = $5 \
            WHERE id = 1;",
            self.theme,
            self.log_level,
            self.tray_icon_theme,
            self.check_for_updates,
            self.selected_view
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    // checks if settings is empty and insert default settings if they not exist, this should be called before app start
    pub async fn init_defaults(pool: &DbPool) -> Result<(), Error> {
        let current_config = query!("SELECT * FROM settings WHERE id = 1;")
            .fetch_optional(pool)
            .await?;
        if current_config.is_none() {
            debug!("No settings found on app init.");
            let mut init_theme = SettingsTheme::Light;
            // check what system theme is currently in use and default to it.
            if dark_light::detect() == dark_light::Mode::Dark {
                debug!("Detected system theme dark, init theme ajusted.");
                init_theme = SettingsTheme::Dark;
            };
            let default_settings = Settings {
                id: None,
                log_level: SettingsLogLevel::Info,
                theme: init_theme,
                tray_icon_theme: TrayIconTheme::Color,
                check_for_updates: true,
                selected_view: None,
            };
            query!(
                "INSERT INTO settings (log_level, theme, tray_icon_theme, check_for_updates, selected_view) VALUES ($1, $2, $3, $4, $5);",
                default_settings.log_level,
                default_settings.theme,
                default_settings.tray_icon_theme,
                default_settings.check_for_updates,
                default_settings.selected_view
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}
