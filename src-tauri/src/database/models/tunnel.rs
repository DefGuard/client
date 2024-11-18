use core::fmt;
use std::time::SystemTime;

use chrono::{NaiveDateTime, Utc};
use defguard_wireguard_rs::host::Peer;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, NoneAsEmptyString};
use sqlx::{query, query_as, query_scalar, Error as SqlxError, SqliteExecutor};

use super::{Id, NoId};
use crate::{
    commands::DateTimeAggregation, database::ActiveConnection, error::Error, CommonConnection,
    CommonConnectionInfo, CommonLocationStats, ConnectionType,
};

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Tunnel<I = NoId> {
    pub id: I,
    pub name: String,
    // user keys
    pub pubkey: String,
    pub prvkey: String,
    // server config
    pub address: String,
    pub server_pubkey: String,
    #[serde_as(as = "NoneAsEmptyString")]
    pub preshared_key: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub allowed_ips: Option<String>,
    // server_address:port
    pub endpoint: String,
    #[serde_as(as = "NoneAsEmptyString")]
    pub dns: Option<String>,
    pub persistent_keep_alive: i64, // New field
    pub route_all_traffic: bool,
    // additional commands
    #[serde_as(as = "NoneAsEmptyString")]
    pub pre_up: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub post_up: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub pre_down: Option<String>,
    #[serde_as(as = "NoneAsEmptyString")]
    pub post_down: Option<String>,
}

impl fmt::Display for Tunnel<Id> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}(ID: {})", self.name, self.id)
    }
}

impl Tunnel<Id> {
    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query!(
            "UPDATE tunnel SET name = $1, pubkey = $2, prvkey = $3, address = $4, \
            server_pubkey = $5, preshared_key = $6, allowed_ips = $7, endpoint = $8, dns = $9, \
            persistent_keep_alive = $10, route_all_traffic = $11, pre_up = $12, post_up = $13, pre_down = $14, post_down = $15 \
            WHERE id = $16;",
            self.name,
            self.pubkey,
            self.prvkey,
            self.address,
            self.server_pubkey,
            self.preshared_key,
            self.allowed_ips,
            self.endpoint,
            self.dns,
            self.persistent_keep_alive,
            self.route_all_traffic,
            self.pre_up,
            self.post_up,
            self.pre_down,
            self.post_down,
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn delete<'e, E>(&self, executor: E) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        Tunnel::delete_by_id(executor, self.id).await?;
        Ok(())
    }

    pub async fn find_by_id<'e, E>(executor: E, tunnel_id: Id) -> Result<Option<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", name, pubkey, prvkey, address, server_pubkey, preshared_key, allowed_ips, endpoint, dns, \
            persistent_keep_alive, route_all_traffic, pre_up, post_up, pre_down, post_down FROM tunnel WHERE id = $1;",
            tunnel_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn all<'e, E>(executor: E) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        let tunnels = query_as!(
            Self,
            "SELECT id \"id: _\", name, pubkey, prvkey, address, server_pubkey, preshared_key, allowed_ips, endpoint, dns, \
            persistent_keep_alive, route_all_traffic, pre_up, post_up, pre_down, post_down FROM tunnel;"
        )
        .fetch_all(executor)
        .await?;
        Ok(tunnels)
    }

    pub async fn find_by_server_public_key<'e, E>(
        executor: E,
        pubkey: &str,
    ) -> Result<Self, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_as!(
           Self,
            "SELECT id \"id: _\", name, pubkey, prvkey, address, server_pubkey, preshared_key, allowed_ips, endpoint, dns, persistent_keep_alive, \
            route_all_traffic, pre_up, post_up, pre_down, post_down \
            FROM tunnel WHERE server_pubkey = $1;",
            pubkey
        )
        .fetch_one(executor)
        .await
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: Id) -> Result<(), Error>
    where
        E: SqliteExecutor<'e>,
    {
        // delete instance
        query!("DELETE FROM tunnel WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }
}

impl Tunnel<NoId> {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        name: String,
        pubkey: String,
        prvkey: String,
        address: String,
        server_pubkey: String,
        preshared_key: Option<String>,
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
            id: NoId,
            name,
            pubkey,
            prvkey,
            address,
            server_pubkey,
            preshared_key,
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

    pub async fn save<'e, E>(self, executor: E) -> Result<Tunnel<Id>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        // Insert a new record when there is no ID
        let result = query!(
            "INSERT INTO tunnel (name, pubkey, prvkey, address, server_pubkey, allowed_ips, preshared_key, \
            endpoint, dns, persistent_keep_alive, route_all_traffic, pre_up, post_up, pre_down, post_down) \
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15) RETURNING id;",
            self.name,
            self.pubkey,
            self.prvkey,
            self.address,
            self.server_pubkey,
            self.allowed_ips,
            self.preshared_key,
            self.endpoint,
            self.dns,
            self.persistent_keep_alive,
            self.route_all_traffic,
            self.pre_up,
            self.post_up,
            self.pre_down,
            self.post_down,
        )
        .fetch_one(executor)
        .await?;

        Ok(Tunnel::<Id> {
            id: result.id,
            name: self.name,
            pubkey: self.pubkey,
            prvkey: self.prvkey,
            address: self.address,
            server_pubkey: self.server_pubkey,
            allowed_ips: self.allowed_ips,
            preshared_key: self.preshared_key,
            endpoint: self.endpoint,
            dns: self.dns,
            persistent_keep_alive: self.persistent_keep_alive,
            route_all_traffic: self.route_all_traffic,
            pre_up: self.pre_up,
            post_up: self.post_up,
            pre_down: self.pre_down,
            post_down: self.post_down,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TunnelStats<I = NoId> {
    id: I,
    pub tunnel_id: Id,
    upload: i64,
    download: i64,
    pub last_handshake: i64,
    collected_at: NaiveDateTime,
    listen_port: u32,
    pub persistent_keepalive_interval: Option<u16>,
}

impl TunnelStats {
    pub async fn get_name<'e, E>(&self, executor: E) -> Result<String, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        query_scalar!("SELECT name FROM tunnel WHERE id = $1;", self.tunnel_id)
            .fetch_one(executor)
            .await
    }
}

impl TunnelStats<NoId> {
    #[must_use]
    pub fn new(
        tunnel_id: Id,
        upload: i64,
        download: i64,
        last_handshake: i64,
        collected_at: NaiveDateTime,
        listen_port: u32,
        persistent_keepalive_interval: Option<u16>,
    ) -> Self {
        TunnelStats {
            id: NoId,
            tunnel_id,
            upload,
            download,
            last_handshake,
            collected_at,
            listen_port,
            persistent_keepalive_interval,
        }
    }

    pub async fn save<'e, E>(self, executor: E) -> Result<TunnelStats<Id>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO tunnel_stats (tunnel_id, upload, download, last_handshake, collected_at, \
            listen_port, persistent_keepalive_interval) \
            VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id \"id!\"",
            self.tunnel_id,
            self.upload,
            self.download,
            self.last_handshake,
            self.collected_at,
            self.listen_port,
            self.persistent_keepalive_interval,
        )
        .fetch_one(executor)
        .await?;

        Ok(TunnelStats::<Id> {
            id,
            tunnel_id: self.tunnel_id,
            upload: self.upload,
            download: self.download,
            last_handshake: self.last_handshake,
            collected_at: self.collected_at,
            listen_port: self.listen_port,
            persistent_keepalive_interval: self.persistent_keepalive_interval,
        })
    }
}

impl TunnelStats<Id> {
    pub async fn all_by_tunnel_id<'e, E>(
        executor: E,
        tunnel_id: Id,
        from: &NaiveDateTime,
        aggregation: &DateTimeAggregation,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: SqliteExecutor<'e>,
    {
        let aggregation = aggregation.fstring();
        let stats = query_as!(
            TunnelStats,
            "WITH cte AS ( \
                SELECT \
                    id, tunnel_id, \
                    COALESCE(upload - LAG(upload) OVER (PARTITION BY tunnel_id ORDER BY collected_at), 0) upload, \
                    COALESCE(download - LAG(download) OVER (PARTITION BY tunnel_id ORDER BY collected_at), 0) download, \
                    last_handshake, strftime($1, collected_at) collected_at, listen_port, persistent_keepalive_interval \
                FROM tunnel_stats ORDER BY collected_at LIMIT -1 OFFSET 1 \
            ) \
            SELECT \
                id, tunnel_id, \
                SUM(MAX(upload, 0)) \"upload!: i64\", \
                SUM(MAX(download, 0)) \"download!: i64\", \
                last_handshake, \
                collected_at \"collected_at!: NaiveDateTime\", \
                listen_port \"listen_port!: u32\", \
                persistent_keepalive_interval \"persistent_keepalive_interval?: u16\" \
            FROM cte \
            WHERE tunnel_id = $2 AND collected_at >= $3 \
            GROUP BY collected_at ORDER BY collected_at",
            aggregation,
            tunnel_id,
            from
        )
        .fetch_all(executor)
        .await?;
        Ok(stats)
    }

    pub async fn latest_by_tunnel_id<'e, E>(
        executor: E,
        tunnel_id: Id,
    ) -> Result<Option<Self>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let res = query_as!(
            TunnelStats::<i64>,
            "SELECT id, tunnel_id, \
           upload \"upload!: i64\", \
           download \"download!: i64\", \
           last_handshake, \
           collected_at \"collected_at!: NaiveDateTime\", \
           listen_port \"listen_port!: u32\", \
           persistent_keepalive_interval \"persistent_keepalive_interval?: u16\" \
           FROM tunnel_stats \
           WHERE tunnel_id=$1 
           ORDER BY last_handshake DESC \
           LIMIT 1
           ",
            tunnel_id
        )
        .fetch_optional(executor)
        .await?;
        Ok(res)
    }
}

pub async fn peer_to_tunnel_stats<'e, E>(
    peer: &Peer,
    listen_port: u32,
    executor: E,
) -> Result<TunnelStats<NoId>, Error>
where
    E: SqliteExecutor<'e>,
{
    let tunnel = Tunnel::find_by_server_public_key(executor, &peer.public_key.to_string()).await?;
    Ok(TunnelStats {
        id: NoId,
        tunnel_id: tunnel.id,
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

#[derive(Debug, Serialize, Clone)]
pub struct TunnelConnection<I = NoId> {
    pub id: I,
    pub tunnel_id: Id,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

impl From<TunnelConnectionInfo> for CommonConnectionInfo {
    fn from(val: TunnelConnectionInfo) -> Self {
        CommonConnectionInfo {
            id: val.id,
            location_id: val.tunnel_id,
            connected_from: val.connected_from,
            start: val.start,
            end: val.end,
            upload: val.upload,
            download: val.download,
        }
    }
}

impl TunnelConnection<Id> {
    pub async fn all_by_tunnel_id<'e, E>(
        executor: E,
        tunnel_id: Id,
    ) -> Result<Vec<TunnelConnection<Id>>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let connections = query_as!(
            TunnelConnection,
            "SELECT id, tunnel_id, connected_from, start, end \
            FROM tunnel_connection WHERE tunnel_id = $1",
            tunnel_id
        )
        .fetch_all(executor)
        .await?;
        Ok(connections)
    }

    pub async fn latest_by_tunnel_id<'e, E>(
        executor: E,
        tunnel_id: Id,
    ) -> Result<Option<TunnelConnection<Id>>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let connection = query_as!(
            TunnelConnection,
            "SELECT id, tunnel_id, connected_from, start, end \
            FROM tunnel_connection WHERE tunnel_id = $1 \
            ORDER BY end DESC LIMIT 1",
            tunnel_id
        )
        .fetch_optional(executor)
        .await?;
        Ok(connection)
    }
}

impl TunnelConnection<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> Result<TunnelConnection<Id>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO tunnel_connection (tunnel_id, connected_from, start, end) \
            VALUES ($1, $2, $3, $4) RETURNING id \"id!\"",
            self.tunnel_id,
            self.connected_from,
            self.start,
            self.end,
        )
        .fetch_one(executor)
        .await?;

        Ok(TunnelConnection::<Id> {
            id,
            tunnel_id: self.tunnel_id,
            connected_from: self.connected_from,
            start: self.start,
            end: self.end,
        })
    }
}

/// Historical connection
#[derive(Debug, Serialize)]
pub struct TunnelConnectionInfo {
    pub id: Id,
    pub tunnel_id: Id,
    pub connected_from: String,
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
    pub upload: Option<i32>,
    pub download: Option<i32>,
}

impl TunnelConnectionInfo {
    pub async fn all_by_tunnel_id<'e, E>(executor: E, tunnel_id: Id) -> Result<Vec<Self>, Error>
    where
        E: SqliteExecutor<'e>,
    {
        // Because we store interface information for given timestamp select last upload and download
        // before connection ended
        // FIXME: Optimize query
        let connections = query_as!(
            TunnelConnectionInfo,
            "SELECT c.id, c.tunnel_id, \
            c.connected_from \"connected_from!\", c.start \"start!\", \
            c.end \"end!\", \
            COALESCE(( \
                SELECT ls.upload \
                FROM tunnel_stats ls \
                WHERE ls.tunnel_id = c.tunnel_id \
                AND ls.collected_at >= c.start \
                AND ls.collected_at <= c.end \
                ORDER BY ls.collected_at DESC LIMIT 1 \
            ), 0) \"upload: _\", \
            COALESCE(( \
                SELECT ls.download \
                FROM tunnel_stats ls \
                WHERE ls.tunnel_id = c.tunnel_id \
                AND ls.collected_at >= c.start \
                AND ls.collected_at <= c.end \
                ORDER BY ls.collected_at DESC LIMIT 1 \
            ), 0) \"download: _\" \
            FROM tunnel_connection c WHERE tunnel_id = $1 \
            ORDER BY start DESC",
            tunnel_id
        )
        .fetch_all(executor)
        .await?;

        Ok(connections)
    }
}

impl From<&ActiveConnection> for TunnelConnection<NoId> {
    fn from(active_connection: &ActiveConnection) -> Self {
        Self {
            id: NoId,
            tunnel_id: active_connection.location_id,
            connected_from: active_connection.connected_from.clone(),
            start: active_connection.start,
            end: Utc::now().naive_utc(),
        }
    }
}

impl From<TunnelConnection<Id>> for CommonConnection<Id> {
    fn from(tunnel_connection: TunnelConnection<Id>) -> Self {
        Self {
            id: tunnel_connection.id,
            location_id: tunnel_connection.tunnel_id, // Assuming you want to map tunnel_id to location_id
            connected_from: tunnel_connection.connected_from,
            start: tunnel_connection.start,
            end: tunnel_connection.end,
            connection_type: ConnectionType::Tunnel, // You need to set the connection_type appropriately based on your logic,
        }
    }
}

impl From<TunnelStats<Id>> for CommonLocationStats<Id> {
    fn from(tunnel_stats: TunnelStats<Id>) -> Self {
        Self {
            id: tunnel_stats.id,
            location_id: tunnel_stats.tunnel_id,
            upload: tunnel_stats.upload,
            download: tunnel_stats.download,
            last_handshake: tunnel_stats.last_handshake,
            collected_at: tunnel_stats.collected_at,
            listen_port: tunnel_stats.listen_port,
            persistent_keepalive_interval: tunnel_stats.persistent_keepalive_interval, // Set the appropriate value
            connection_type: ConnectionType::Tunnel,
        }
    }
}
