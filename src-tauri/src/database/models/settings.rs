use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sqlx::{query, FromRow, Type};
use struct_patch::Patch;
use strum::EnumString;

use crate::{database::DbPool, error::Error};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type, EnumString)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type, EnumString)]
#[sqlx(type_name = "tray_icon_theme", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TrayIconTheme {
    Color,
    White,
    Black,
    Gray,
}

#[derive(FromRow, Debug, Serialize, Deserialize, Patch)]
#[patch_derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(skip)]
    pub id: Option<i64>,
    pub theme: SettingsTheme,
    pub log_level: SettingsLogLevel,
    pub tray_icon_theme: TrayIconTheme,
}

impl Settings {
    pub async fn get(pool: &DbPool) -> Result<Self, Error> {
        let query_res = query!(
            r#"
            SELECT * FROM settings WHERE id = 1
        "#
        )
        .fetch_one(pool)
        .await?;
        let settings = Self {
            id: Some(query_res.id),
            log_level: SettingsLogLevel::from_str(&query_res.log_level)?,
            theme: SettingsTheme::from_str(&query_res.theme)?,
            tray_icon_theme: TrayIconTheme::from_str(&query_res.tray_icon_theme)?,
        };
        Ok(settings)
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        query!(
            r#"
            UPDATE settings
            SET theme = $1, log_level = $2, tray_icon_theme = $3
            WHERE id = 1;
        "#,
            self.theme,
            self.log_level,
            self.tray_icon_theme
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    // checks if settings is empty and insert default settings if they not exist, this should be called before app start
    pub async fn init_defaults(pool: &DbPool) -> Result<(), Error> {
        let current_config = query!(
            r#"
            SELECT * FROM settings WHERE id = 1;
        "#
        )
        .fetch_optional(pool)
        .await?;
        match current_config {
            Some(_) => Ok(()),
            None => {
                let default_settings = Settings {
                    id: None,
                    log_level: SettingsLogLevel::Info,
                    theme: SettingsTheme::Light,
                    tray_icon_theme: TrayIconTheme::Color,
                };
                query!(
                    r#"
                INSERT INTO settings (log_level, theme, tray_icon_theme)
                VALUES ($1, $2, $3);
            "#,
                    default_settings.log_level,
                    default_settings.theme,
                    default_settings.tray_icon_theme,
                )
                .execute(pool)
                .await?;
                Ok(())
            }
        }
    }
}
