use crate::database::DbPool;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Error as SqlxError, FromRow};

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
    pub allowed_ips: String,
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
        allowed_ips: String,
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
}
