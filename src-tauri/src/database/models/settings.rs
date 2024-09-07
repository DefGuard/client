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

impl Default for Settings {
    fn default() -> Self {
        Self {
            id: None,
            log_level: SettingsLogLevel::Info,
            theme: SettingsTheme::Light,
            tray_icon_theme: TrayIconTheme::Color,
            check_for_updates: true,
            selected_view: None,
        }
    }
}

impl Settings {
    pub async fn get<'e, E>(executor: E) -> Result<Self, Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
        let query_res = query!("SELECT * FROM settings WHERE id = 1;")
            .fetch_one(executor)
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

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
    {
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
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Checks if settings are empty and inserts default settings if they not exist, this should be called before app start
    pub async fn init_defaults(pool: &DbPool) -> Result<(), Error> {
        let current_config = query!("SELECT * FROM settings WHERE id = 1;")
            .fetch_optional(pool)
            .await?;
        if current_config.is_none() {
            debug!("No settings found on app init, inserting defaults.");
            // check what system theme is currently in use and default to it.
            let theme = match dark_light::detect() {
                dark_light::Mode::Default => SettingsTheme::Light,
                dark_light::Mode::Light => SettingsTheme::Light,
                dark_light::Mode::Dark => {
                    debug!("Detected system theme dark, init theme adjusted.");
                    SettingsTheme::Dark
                }
            };
            let settings = Settings {
                theme,
                ..Default::default()
            };
            query!(
                "INSERT INTO settings (log_level, theme, tray_icon_theme, check_for_updates, selected_view) VALUES ($1, $2, $3, $4, $5);",
                settings.log_level,
                settings.theme,
                settings.tray_icon_theme,
                settings.check_for_updates,
                settings.selected_view
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}
