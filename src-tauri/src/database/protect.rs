use crate::{
    database::{
        Connection, Instance, Location, LocationStats, Settings, Tunnel, TunnelConnection,
        TunnelStats, WireguardKeys,
    },
    error::Error,
};
use chrono::NaiveDateTime;
use tauri::AppHandle;

use super::{init_db_connection, DbPool};

// Write data from unprotected db to protected db
pub(crate) async fn protect_db(
    app: &AppHandle,
    source_pool: &DbPool,
    password: &str,
) -> Result<(), Error> {
    info!("Opening conn to protected DB");
    let protected_pool = init_db_connection(app, Some(password.into()), None).await?;
    info!("Connection open");
    info!("Attempting to copy data from unprotected database.");
    let instance_data = sqlx::query_as!(Instance, "SELECT * FROM instance;")
        .fetch_all(source_pool)
        .await?;
    let wireguard_keys_data = sqlx::query_as!(WireguardKeys, "SELECT * FROM wireguard_keys")
        .fetch_all(source_pool)
        .await?;
    let location_data = sqlx::query_as!(Location, "SELECT * FROM location")
        .fetch_all(source_pool)
        .await?;
    let connection_data = sqlx::query_as!(Connection, "SELECT * FROM connection")
        .fetch_all(source_pool)
        .await?;
    let location_stats_data = sqlx::query_as!(
        LocationStats,
        r#"SELECT
              id,
              location_id,
              upload as "upload!: i64",
            	download as "download!: i64",
            	last_handshake,
            	collected_at as "collected_at!: NaiveDateTime",
            	listen_port as "listen_port!: u32",
            	persistent_keepalive_interval as "persistent_keepalive_interval?: u16" 
              FROM location_stats;
    "#
    )
    .fetch_all(source_pool)
    .await?;
    let tunnel_data = sqlx::query_as!(Tunnel, "SELECT * from tunnel;")
        .fetch_all(source_pool)
        .await?;
    let tunnel_connection_data =
        sqlx::query_as!(TunnelConnection, "SELECT * from tunnel_connection;")
            .fetch_all(source_pool)
            .await?;
    let tunnel_stats_data = sqlx::query_as!(
        TunnelStats,
        r#"SELECT 
          id,
          tunnel_id,
          upload as "upload!: i64",
          download as "download!: i64",
          last_handshake,
          collected_at as "collected_at!: NaiveDateTime",
          listen_port as "listen_port!: u32",
          persistent_keepalive_interval as "persistent_keepalive_interval?: u16"  
     FROM tunnel_stats;"#
    )
    .fetch_all(source_pool)
    .await?;
    let mut settings_data = Settings::get(source_pool).await?;
    info!("Data copied. Attempting to write into protected database");

    // save in transaction into protected
    let mut tx = protected_pool.begin().await?;
    for row in instance_data.iter() {
        sqlx::query!(
            r#"INSERT INTO instance
            (id, uuid, name, url, proxy_url, username) 
            VALUES($1, $2, $3, $4, $5, $6);"#,
            row.id,
            row.uuid,
            row.name,
            row.url,
            row.proxy_url,
            row.username
        )
        .execute(&mut *tx)
        .await?;
    }
    for row in wireguard_keys_data.iter() {
        sqlx::query!(
            r#"INSERT INTO wireguard_keys 
            (id, instance_id, pubkey, prvkey) 
            VALUES($1,$2,$3,$4);"#,
            row.id,
            row.instance_id,
            row.pubkey,
            row.prvkey,
        )
        .execute(&mut *tx)
        .await?;
    }
    for row in location_data.iter() {
        sqlx::query!(
          "INSERT INTO location \
          (id, instance_id, network_id, name, address, pubkey, endpoint, allowed_ips, dns, route_all_traffic, mfa_enabled, keepalive_interval) \
          VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12);",
          row.id,
          row.instance_id,
          row.network_id,
          row.name,
          row.address,
          row.pubkey,
          row.endpoint,
          row.allowed_ips,
          row.dns,
          row.route_all_traffic,
          row.mfa_enabled,
          row.keepalive_interval
        ).execute(&mut *tx).await?;
    }
    for row in connection_data.iter() {
        sqlx::query!(
            "INSERT INTO connection \
            (id, location_id, connected_from, start, end) \
            VALUES($1,$2,$3,$4,$5);",
            row.id,
            row.location_id,
            row.connected_from,
            row.start,
            row.end
        )
        .execute(&mut *tx)
        .await?;
    }
    for row in location_stats_data.iter() {
        sqlx::query!("INSERT INTO location_stats \
        (id, location_id, upload, download, last_handshake, collected_at, listen_port, persistent_keepalive_interval) \
        VALUES($1,$2,$3,$4,$5,$6,$7,$8);",
        row.id,
        row.location_id,
      row.upload,
      row.download,row.last_handshake,row.collected_at,
      row.listen_port,row.persistent_keepalive_interval).execute(&mut *tx).await?;
    }
    for row in tunnel_data.iter() {
        sqlx::query!("INSERT INTO tunnel \
        (id, name, pubkey, prvkey, server_pubkey, allowed_ips, endpoint, dns, \
        route_all_traffic, persistent_keep_alive, pre_up, post_up, pre_down, post_down, preshared_key) \
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15);",
        row.id,
        row.name,
        row.pubkey,
        row.prvkey,
        row.server_pubkey,
        row.allowed_ips,
        row.endpoint,
        row.dns,
        row.route_all_traffic,
        row.persistent_keep_alive,
        row.pre_up,
        row.post_up,
        row.pre_down,
        row.post_down,
        row.preshared_key,
      ).execute(&mut *tx).await?;
    }
    for row in tunnel_connection_data.iter() {
        sqlx::query!(
            "INSERT INTO tunnel_connection \
        (id, tunnel_id, connected_from, start, end) \
        VALUES($1,$2,$3,$4,$5);",
            row.id,
            row.tunnel_id,
            row.connected_from,
            row.start,
            row.end
        )
        .execute(&mut *tx)
        .await?;
    }
    for row in tunnel_stats_data.iter() {
        sqlx::query!("INSERT INTO tunnel_stats \
        (id, tunnel_id, upload, download, last_handshake, collected_at, listen_port, persistent_keepalive_interval) \
        VALUES($1,$2,$3,$4,$5,$6,$7,$8);",
        row.id,
        row.tunnel_id,
        row.upload,
        row.download,
        row.last_handshake,
        row.collected_at,
        row.listen_port,
        row.persistent_keepalive_interval)
        .execute(&mut *tx).await?;
    }
    settings_data.save(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
