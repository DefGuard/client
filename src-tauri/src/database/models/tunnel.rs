use crate::{commands::DateTimeAggregation, database::{DbPool, ActiveConnection}, error::Error};
use chrono::{NaiveDateTime, Utc};
use defguard_wireguard_rs::host::Peer;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Error as SqlxError, FromRow};
use std::time::SystemTime;

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: Option<i64>,
    pub name: String,
    // user keys
    pub pubkey: String,
    pub prvkey: String,
    // server config
    pub address: String,
    pub server_pubkey: String,
    pub allowed_ips: Option<String>,
    // server_address:port
    pub endpoint: String,
    pub dns: Option<String>,
    pub persistent_keep_alive: i64, // New field
    pub route_all_traffic: bool,
    // additional commands
    pub pre_up: Option<String>,
    pub post_up: Option<String>,
    pub pre_down: Option<String>,
    pub post_down: Option<String>,
}

impl Tunnel {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        name: String,
        pubkey: String,
        prvkey: String,
        address: String,
        server_pubkey: String,
        allowed_ips: Option<String>,
        endpoint: String,
        dns: Option<String>,
        persistent_keep_alive: i64,
        route_all_traffic: bool,
        pre_up: Option<String>,
        post_up: Option<String>,
        pre_down: Option<String>,
        post_down: Option<String>,
    ) -> Self {
        Tunnel {
            id: None,
            name,
            pubkey,
            prvkey,
            address,
            server_pubkey,
            allowed_ips,
            endpoint,
            dns,
            persistent_keep_alive,
            route_all_traffic,
            pre_up,
            post_up,
            pre_down,
            post_down,
        }
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), SqlxError> {
        match self.id {
            None => {
                // Insert a new record when there is no ID
                let result = query!(
                    "INSERT INTO tunnel (name, pubkey, prvkey, address, server_pubkey, allowed_ips, \
                    endpoint, dns, persistent_keep_alive, route_all_traffic, pre_up, post_up, pre_down, post_down) \
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id;",
                    self.name,
                    self.pubkey,
                    self.prvkey,
                    self.address,
                    self.server_pubkey,
                    self.allowed_ips,
                    self.endpoint,
                    self.dns,
                    self.persistent_keep_alive,
                    self.route_all_traffic,
                    self.pre_up,
                    self.post_up,
                    self.pre_down,
                    self.post_down,
                )
                .fetch_one(pool)
                .await?;
                self.id = Some(result.id);
            }
            Some(id) => {
                // Update the existing record when there is an ID
                query!(
                    "UPDATE tunnel SET name = $1, pubkey = $2, prvkey = $3, address = $4, \
                    server_pubkey = $5, allowed_ips = $6, endpoint = $7, dns = $8, \
                    persistent_keep_alive = $9, route_all_traffic = $10, pre_up = $11, post_up = $12, pre_down = $13, post_down = $14 \
                    WHERE id = $15;",
                    self.name,
                    self.pubkey,
                    self.prvkey,
                    self.address,
                    self.server_pubkey,
                    self.allowed_ips,
                    self.endpoint,
                    self.dns,
                    self.persistent_keep_alive,
                    self.route_all_traffic,
                    self.pre_up,
                    self.post_up,
                    self.pre_down,
                    self.post_down,
                    id,
                )
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn find_by_id(pool: &DbPool, tunnel_id: i64) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", name, pubkey, prvkey, address, server_pubkey, allowed_ips, endpoint, dns, \
            persistent_keep_alive, route_all_traffic, pre_up, post_up, pre_down, post_down FROM tunnel WHERE id = $1;",
            tunnel_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn all(pool: &DbPool) -> Result<Vec<Self>, SqlxError> {
        let tunnels = query_as!(
            Self,
            "SELECT id \"id?\", name, pubkey, prvkey, address, server_pubkey, allowed_ips, endpoint, dns, \
            persistent_keep_alive, route_all_traffic, pre_up, post_up, pre_down, post_down FROM tunnel;"
        )
        .fetch_all(pool)
        .await?;
        Ok(tunnels)
    }
    pub async fn find_by_public_key(pool: &DbPool, pubkey: &str) -> Result<Self, SqlxError> {
        query_as!(
            Tunnel,
            "SELECT id \"id?\", name, pubkey, prvkey, address, server_pubkey, allowed_ips, endpoint, dns, persistent_keep_alive, 
            route_all_traffic, pre_up, post_up, pre_down, post_down \
            FROM tunnel WHERE pubkey = $1;",
            pubkey
        )
        .fetch_one(pool)
        .await
    }
}

#[derive(FromRow, Debug, Serialize, Deserialize)]
pub struct TunnelStats {
    id: Option<i64>,
    tunnel_id: i64,
    upload: i64,
    download: i64,
    last_handshake: i64,
    collected_at: NaiveDateTime,
    listen_port: u32,
    persistent_keepalive_interval: Option<u16>,
}

impl TunnelStats {
    #[must_use]
    pub fn new(
        tunnel_id: i64,
        upload: i64,
        download: i64,
        last_handshake: i64,
        collected_at: NaiveDateTime,
        listen_port: u32,
        persistent_keepalive_interval: Option<u16>,
    ) -> Self {
        TunnelStats {
            id: None,
            tunnel_id,
            upload,
            download,
            last_handshake,
            collected_at,
            listen_port,
            persistent_keepalive_interval,
        }
    }

    pub async fn save(&mut self, pool: &DbPool) -> Result<(), SqlxError> {
        let result = query!(
            "INSERT INTO tunnel_stats (tunnel_id, upload, download, last_handshake, collected_at, listen_port, persistent_keepalive_interval) \
            VALUES ($1, $2, $3, $4, $5, $6, $7) \
            RETURNING id;",
            self.tunnel_id,
            self.upload,
            self.download,
            self.last_handshake,
            self.collected_at,
            self.listen_port,
            self.persistent_keepalive_interval,
        )
        .fetch_one(pool)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }

    pub async fn all_by_tunnel_id(
        pool: &DbPool,
        tunnel_id: i64,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<Self>, SqlxError> {
        let aggregation = aggregation.fstring();
        let stats = query_as!(
            TunnelStats,
            r#"
            WITH cte AS (
                SELECT
                    id, tunnel_id,
                    COALESCE(upload - LAG(upload) OVER (PARTITION BY tunnel_id ORDER BY collected_at), 0) as upload,
                    COALESCE(download - LAG(download) OVER (PARTITION BY tunnel_id ORDER BY collected_at), 0) as download,
                    last_handshake, strftime($1, collected_at) as collected_at, listen_port, persistent_keepalive_interval
                FROM tunnel_stats
                ORDER BY collected_at
                LIMIT -1 OFFSET 1
            )
            SELECT
                id, tunnel_id,
                SUM(MAX(upload, 0)) as "upload!: i64",
                SUM(MAX(download, 0)) as "download!: i64",
                last_handshake,
                collected_at as "collected_at!: NaiveDateTime",
                listen_port as "listen_port!: u32",
                persistent_keepalive_interval as "persistent_keepalive_interval?: u16"
            FROM cte
            WHERE tunnel_id = $2
            AND collected_at >= $3
            GROUP BY collected_at
            ORDER BY collected_at;
            "#,
            aggregation,
            tunnel_id,
            from
        )
        .fetch_all(pool)
        .await?;
        Ok(stats)
    }
}
pub async fn peer_to_tunnel_stats(
    peer: &Peer,
    listen_port: u32,
    pool: &DbPool,
) -> Result<TunnelStats, Error> {
    let tunnel = Tunnel::find_by_public_key(pool, &peer.public_key.to_string()).await?;
    Ok(TunnelStats {
        id: None,
        tunnel_id: tunnel.id.unwrap(),
        upload: peer.tx_bytes as i64,
        download: peer.rx_bytes as i64,
        last_handshake: peer.last_handshake.map_or(0, |ts| {
            ts.duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_secs() as i64)
        }),
        collected_at: Utc::now().naive_utc(),
        listen_port,
        persistent_keepalive_interval: peer.persistent_keepalive_interval,
    })
}

#[derive(FromRow, Debug, Serialize, Clone)]
pub struct TunnelConnection {
    pub id: Option<i64>,
    pub tunnel_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl TunnelConnection {
    pub async fn save(&mut self, pool: &DbPool) -> Result<(), Error> {
        let result = query!(
            "INSERT INTO tunnel_connection (tunnel_id, connected_from, start, end) \
            VALUES ($1, $2, $3, $4) \
            RETURNING id;",
            self.tunnel_id,
            self.connected_from,
            self.start,
            self.end,
        )
        .fetch_one(pool)
        .await?;
        self.id = Some(result.id);
        Ok(())
    }

    pub async fn all_by_tunnel_id(pool: &DbPool, tunnel_id: i64) -> Result<Vec<Self>, Error> {
        let connections = query_as!(
            TunnelConnection,
            r#"
            SELECT id, tunnel_id, connected_from, start, end 
            FROM tunnel_connection
            WHERE tunnel_id = $1
            "#,
            tunnel_id
        )
        .fetch_all(pool)
        .await?;
        Ok(connections)
    }

    pub async fn latest_by_tunnel_id(pool: &DbPool, tunnel_id: i64) -> Result<Option<Self>, Error> {
        let connection = query_as!(
            TunnelConnection,
            r#"
            SELECT id, tunnel_id, connected_from, start, end
            FROM tunnel_connection
            WHERE tunnel_id = $1
            ORDER BY end DESC
            LIMIT 1
            "#,
            tunnel_id
        )
        .fetch_optional(pool)
        .await?;
        Ok(connection)
    }
}

/// Historical connection
#[derive(FromRow, Debug, Serialize)]
pub struct TunnelConnectionInfo {
    pub id: i64,
    pub tunnel_id: i64,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub upload: Option<i32>,
    pub download: Option<i32>,
}

impl TunnelConnectionInfo {
    pub async fn all_by_tunnel_id(pool: &DbPool, tunnel_id: i64) -> Result<Vec<Self>, Error> {
        // Because we store interface information for given timestamp select last upload and download
        // before connection ended
        // FIXME: Optimize query
        let connections = query_as!(
            TunnelConnectionInfo,
            r#"
              SELECT
                  c.id as "id!",
                  c.tunnel_id as "tunnel_id!",
                  c.connected_from as "connected_from!",
                  c.start as "start!",
                  c.end as "end!",
                  COALESCE((
                      SELECT ls.upload
                      FROM tunnel_stats AS ls
                      WHERE ls.tunnel_id = c.tunnel_id
                      AND ls.collected_at >= c.start
                      AND ls.collected_at <= c.end
                      ORDER BY ls.collected_at DESC
                      LIMIT 1
                  ), 0) as "upload: _",
                  COALESCE((
                      SELECT ls.download
                      FROM tunnel_stats AS ls
                      WHERE ls.tunnel_id = c.tunnel_id
                      AND ls.collected_at >= c.start
                      AND ls.collected_at <= c.end
                      ORDER BY ls.collected_at DESC
                      LIMIT 1
                  ), 0) as "download: _"
              FROM tunnel_connection AS c WHERE tunnel_id = $1
              ORDER BY start DESC;
            "#,
            tunnel_id
        )
        .fetch_all(pool)
        .await?;

        Ok(connections)
    }
}
impl From<ActiveConnection> for TunnelConnection {
    fn from(active_connection: ActiveConnection) -> Self {
        TunnelConnection {
            id: None,
            tunnel_id: active_connection.location_id,
            connected_from: active_connection.connected_from,
            start: active_connection.start,
            end: Utc::now().naive_utc(),
        }
    }
}
