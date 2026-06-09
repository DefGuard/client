use std::time::SystemTime;

use chrono::{NaiveDateTime, Utc};
use defguard_wireguard_rs::peer::Peer;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, query_scalar, SqliteExecutor};

use super::{location::Location, Id, NoId, PURGE_DURATION};
use crate::{CommonLocationStats, ConnectionType, DateTimeAggregation};

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationStats<I = NoId> {
    id: I,
    pub location_id: Id,
    upload: i64,
    download: i64,
    pub last_handshake: i64,
    pub collected_at: NaiveDateTime,
    listen_port: u32,
    pub persistent_keepalive_interval: Option<u16>,
}

impl From<LocationStats<Id>> for CommonLocationStats<Id> {
    fn from(location_stats: LocationStats<Id>) -> Self {
        CommonLocationStats {
            id: location_stats.id,
            location_id: location_stats.location_id,
            upload: location_stats.upload,
            download: location_stats.download,
            last_handshake: location_stats.last_handshake,
            collected_at: location_stats.collected_at,
            listen_port: location_stats.listen_port,
            persistent_keepalive_interval: location_stats.persistent_keepalive_interval,
            connection_type: ConnectionType::Location,
        }
    }
}

pub async fn peer_to_location_stats<'e, E>(
    peer: &Peer,
    listen_port: u32,
    executor: E,
) -> sqlx::Result<LocationStats<NoId>>
where
    E: SqliteExecutor<'e>,
{
    let location = Location::find_by_public_key(executor, &peer.public_key.to_string()).await?;
    Ok(LocationStats::new(
        location.id,
        peer.tx_bytes.cast_signed(),
        peer.rx_bytes.cast_signed(),
        peer.last_handshake.map_or(0, |ts| {
            ts.duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs().cast_signed())
        }),
        listen_port,
        peer.persistent_keepalive_interval,
    ))
}

impl LocationStats {
    // Although not used on macOS, allow dead code for `sqlx prepare`.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub async fn get_name<'e, E>(&self, executor: E) -> sqlx::Result<String>
    where
        E: SqliteExecutor<'e>,
    {
        query_scalar!("SELECT name FROM location WHERE id = $1", self.location_id)
            .fetch_one(executor)
            .await
    }
}

impl LocationStats<NoId> {
    #[must_use]
    pub fn new(
        location_id: Id,
        upload: i64,
        download: i64,
        last_handshake: i64,
        listen_port: u32,
        persistent_keepalive_interval: Option<u16>,
    ) -> Self {
        LocationStats {
            id: NoId,
            location_id,
            upload,
            download,
            last_handshake,
            collected_at: Utc::now().naive_utc(),
            listen_port,
            persistent_keepalive_interval,
        }
    }

    pub async fn save<'e, E>(self, executor: E) -> sqlx::Result<LocationStats<Id>>
    where
        E: SqliteExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO location_stats (location_id, upload, download, last_handshake, \
            collected_at, listen_port, persistent_keepalive_interval) \
            VALUES ($1, $2, $3, $4, $5, $6, $7) \
            RETURNING id \"id!\"",
            self.location_id,
            self.upload,
            self.download,
            self.last_handshake,
            self.collected_at,
            self.listen_port,
            self.persistent_keepalive_interval,
        )
        .fetch_one(executor)
        .await?;

        Ok(LocationStats::<Id> {
            id,
            location_id: self.location_id,
            upload: self.upload,
            download: self.download,
            last_handshake: self.last_handshake,
            collected_at: self.collected_at,
            listen_port: self.listen_port,
            persistent_keepalive_interval: self.persistent_keepalive_interval,
        })
    }
}

impl LocationStats<Id> {
    pub async fn all_by_location_id<'e, E>(
        executor: E,
        location_id: Id,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
        limit: Option<i32>,
    ) -> sqlx::Result<Vec<Self>>
    where
        E: SqliteExecutor<'e>,
    {
        let aggregation = aggregation.fstring();
        // SQLite: If the LIMIT expression evaluates to a negative value,
        // then there is no upper bound on the number of rows returned
        let query_limit = limit.unwrap_or(-1);
        let stats = query_as!(
            LocationStats,
            "WITH cte AS (\
            SELECT id, location_id, \
            COALESCE(upload - LAG(upload) OVER (PARTITION BY location_id ORDER BY collected_at), 0) upload, \
            COALESCE(download - LAG(download) OVER (PARTITION BY location_id ORDER BY collected_at), 0) download, \
            last_handshake, strftime($1, collected_at) collected_at, listen_port, persistent_keepalive_interval \
            FROM location_stats ORDER BY collected_at LIMIT -1 OFFSET 1) \
            SELECT id, location_id, \
           	SUM(MAX(upload, 0)) \"upload!: i64\", \
           	SUM(MAX(download, 0)) \"download!: i64\", \
           	last_handshake, \
           	collected_at \"collected_at!: NaiveDateTime\", \
           	listen_port \"listen_port!: u32\", \
           	persistent_keepalive_interval \"persistent_keepalive_interval?: u16\" \
            FROM cte WHERE location_id = $2 AND collected_at >= $3 \
            GROUP BY collected_at ORDER BY collected_at LIMIT $4",
            aggregation,
            location_id,
            from,
            query_limit
        )
        .fetch_all(executor)
        .await?;
        Ok(stats)
    }

    pub async fn latest_by_download_change<'e, E>(
        executor: E,
        location_id: Id,
    ) -> sqlx::Result<Option<Self>>
    where
        E: SqliteExecutor<'e>,
    {
        let res = query_as!(
            LocationStats::<Id>,
            "WITH prev_download AS (
              SELECT download
              FROM location_stats
              WHERE location_id = $1
              ORDER BY collected_at DESC
              LIMIT 1 OFFSET 1
          )
          SELECT ls.id \"id!: i64\",
              ls.location_id,
              ls.upload \"upload!: i64\",
              ls.download \"download!: i64\",
              ls.last_handshake,
              ls.collected_at \"collected_at!: NaiveDateTime\",
              ls.listen_port \"listen_port!: u32\",
              ls.persistent_keepalive_interval \"persistent_keepalive_interval?: u16\"
          FROM location_stats ls
          LEFT JOIN prev_download pd
          WHERE ls.location_id = $1
          AND (pd.download IS NULL OR ls.download != pd.download)
          ORDER BY ls.collected_at DESC
          LIMIT 1",
            location_id
        )
        .fetch_optional(executor)
        .await?;
        Ok(res)
    }

    /// Purge old statistics.
    pub async fn purge<'e, E>(executor: E) -> sqlx::Result<()>
    where
        E: SqliteExecutor<'e>,
    {
        debug!("Purging location statistics.");

        let past = (Utc::now() - PURGE_DURATION).naive_utc();
        query!("DELETE FROM location_stats WHERE collected_at < $1", past)
            .execute(executor)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;

    use super::*;
    use crate::database::models::{
        instance::{ClientTrafficPolicy, Instance},
        location::{LocationMfaMode, ServiceLocationMode},
    };

    async fn seed_location(pool: &SqlitePool) -> Id {
        let instance = Instance {
            id: NoId,
            name: "instance".into(),
            uuid: "uuid-1".into(),
            url: "https://core.example".into(),
            proxy_url: "https://proxy.example".into(),
            username: "alice".into(),
            token: None,
            client_traffic_policy: ClientTrafficPolicy::None,
            enterprise_enabled: false,
            openid_display_name: None,
        }
        .save(pool)
        .await
        .unwrap();

        Location {
            id: NoId,
            instance_id: instance.id,
            network_id: 1,
            name: "loc".into(),
            address: "10.0.0.2/24".into(),
            pubkey: "pk".into(),
            endpoint: "1.2.3.4:51820".into(),
            allowed_ips: "0.0.0.0/0".into(),
            dns: None,
            route_all_traffic: false,
            keepalive_interval: 25,
            location_mfa_mode: LocationMfaMode::Disabled,
            service_location_mode: ServiceLocationMode::Disabled,
            mfa_method: None,
            posture_check_required: false,
        }
        .save(pool)
        .await
        .unwrap()
        .id
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_location_stats_save_round_trip(pool: SqlitePool) {
        let location_id = seed_location(&pool).await;

        let stats = LocationStats::new(location_id, 100, 200, 1_700_000_000, 51820, Some(25))
            .save(&pool)
            .await
            .unwrap();

        // A real row id is returned on insert.
        assert!(stats.id > 0);
        assert_eq!(stats.location_id, location_id);
    }
}
