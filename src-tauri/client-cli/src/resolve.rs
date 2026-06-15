use defguard_core::database::{
    models::{instance::Instance, location::Location, tunnel::Tunnel, Id},
    DbPool,
};

use crate::state::CliError;

/// The user's target specification, parsed from CLI arguments.
pub struct TargetSpec {
    pub name: Option<String>,
    pub tunnel: bool,
    pub id: Option<Id>,
    pub instance: Option<String>,
}

/// A resolved connection target.
pub enum ResolvedTarget {
    Location(Location<Id>),
    Tunnel(Tunnel<Id>),
}

/// Resolve a target for the `connect` command.
pub async fn resolve_connect_target(
    spec: &TargetSpec,
    pool: &DbPool,
) -> Result<ResolvedTarget, CliError> {
    // --id fast path
    if let Some(id) = spec.id {
        if spec.tunnel {
            let tun = Tunnel::find_by_id(pool, id)
                .await?
                .ok_or_else(|| CliError::NotFound(format!("No tunnel with id {id}")))?;
            return Ok(ResolvedTarget::Tunnel(tun));
        }
        if let Some(loc) = Location::find_by_id(pool, id).await? {
            return Ok(ResolvedTarget::Location(loc));
        }
        if let Some(tun) = Tunnel::find_by_id(pool, id).await? {
            return Ok(ResolvedTarget::Tunnel(tun));
        }
        return Err(CliError::NotFound(format!(
            "No location or tunnel with id {id}"
        )));
    }

    // Named target
    if let Some(ref name) = spec.name {
        return resolve_named(name, spec.tunnel, spec.instance.as_deref(), pool).await;
    }

    // No-arg: pick sole location
    resolve_sole_location(pool).await
}

/// Resolve a target for the `disconnect` command.
///
/// No-arg / --all resolution is handled directly in the disconnect handler
/// using `active_state`; this function is only called when a target is named.
pub async fn resolve_disconnect_target(
    spec: &TargetSpec,
    pool: &DbPool,
) -> Result<ResolvedTarget, CliError> {
    resolve_connect_target(spec, pool).await
}

async fn resolve_named(
    name: &str,
    tunnel_only: bool,
    instance_filter: Option<&str>,
    pool: &DbPool,
) -> Result<ResolvedTarget, CliError> {
    if tunnel_only {
        let tunnels = Tunnel::find_by_name(pool, name).await?;
        return match tunnels.len() {
            0 => Err(CliError::NotFound(format!("Tunnel '{name}' not found"))),
            1 => Ok(ResolvedTarget::Tunnel(
                tunnels
                    .into_iter()
                    .next()
                    .expect("exactly one tunnel expected after length check"),
            )),
            _ => Err(CliError::NotFound(format!(
                "Multiple tunnels named '{name}'"
            ))),
        };
    }

    // Fetch matching locations by name.  Instance filter is applied in Rust
    // since cross-instance ambiguity is a business rule, not a query concern.
    let loc_matches: Vec<Location<Id>> = if let Some(inst_name) = instance_filter {
        let inst = Instance::find_by_name(pool, inst_name)
            .await?
            .ok_or_else(|| CliError::NotFound(format!("Instance '{inst_name}' not found")))?;
        Location::find_by_name(pool, name)
            .await?
            .into_iter()
            .filter(|l| l.instance_id == inst.id)
            .collect()
    } else {
        Location::find_by_name(pool, name).await?
    };

    let tun_matches = Tunnel::find_by_name(pool, name).await?;

    match (loc_matches.len(), tun_matches.len()) {
        (0, 0) => Err(CliError::NotFound(format!("'{name}' not found"))),
        (1, 0) => Ok(ResolvedTarget::Location(
            loc_matches
                .into_iter()
                .next()
                .expect("exactly one location expected after length check"),
        )),
        (0, 1) => Ok(ResolvedTarget::Tunnel(
            tun_matches
                .into_iter()
                .next()
                .expect("exactly one tunnel expected after length check"),
        )),
        (1, 1) => Err(CliError::NotFound(format!(
            "'{name}' matches both a location and a tunnel. Use --tunnel."
        ))),
        (n, 0) if n > 1 => Err(CliError::NotFound(format!(
            "'{name}' exists in multiple instances. Use --instance to pick one."
        ))),
        _ => Err(CliError::NotFound(format!(
            "'{name}' matches multiple targets"
        ))),
    }
}

async fn resolve_sole_location(pool: &DbPool) -> Result<ResolvedTarget, CliError> {
    let locations = Location::all(pool, false).await?;
    match locations.len() {
        0 => Err(CliError::NotEnrolled(
            "No locations configured. Enroll an instance first.".into(),
        )),
        1 => Ok(ResolvedTarget::Location(
            locations
                .into_iter()
                .next()
                .expect("exactly one location expected after length check"),
        )),
        _ => Err(CliError::NotFound(
            "Multiple locations available. Specify a name.".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use defguard_core::database::models::{
        instance::{ClientTrafficPolicy, Instance},
        location::{Location, LocationMfaMode, ServiceLocationMode},
        tunnel::Tunnel,
        Id, NoId,
    };

    use super::*;

    fn sample_instance(name: &str) -> Instance<NoId> {
        Instance {
            id: NoId,
            name: name.into(),
            uuid: format!("uuid-{name}"),
            url: format!("https://{name}.example"),
            proxy_url: format!("https://proxy.{name}.example"),
            username: "alice".into(),
            token: None,
            client_traffic_policy: ClientTrafficPolicy::None,
            enterprise_enabled: false,
            openid_display_name: None,
        }
    }

    fn sample_location(name: &str, instance_id: Id) -> Location<NoId> {
        Location {
            id: NoId,
            instance_id,
            network_id: 1,
            name: name.into(),
            address: "10.0.0.2/24".into(),
            pubkey: format!("pk-loc-{name}"),
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
    }

    fn sample_tunnel(name: &str) -> Tunnel<NoId> {
        Tunnel {
            id: NoId,
            name: name.into(),
            pubkey: format!("pk-tun-{name}"),
            prvkey: format!("prvk-tun-{name}"),
            address: "10.1.0.2/24".into(),
            server_pubkey: format!("spk-tun-{name}"),
            preshared_key: None,
            allowed_ips: Some("0.0.0.0/0".into()),
            endpoint: "5.6.7.8:51820".into(),
            dns: None,
            persistent_keep_alive: 25,
            route_all_traffic: false,
            pre_up: None,
            post_up: None,
            pre_down: None,
            post_down: None,
        }
    }

    /// Unwrap helper: avoids the `Debug` bound on `ResolvedTarget`.
    fn expect_ok(result: Result<ResolvedTarget, CliError>) -> ResolvedTarget {
        match result {
            Ok(r) => r,
            Err(e) => panic!("expected Ok, got Err: {e}"),
        }
    }

    fn expect_err(result: Result<ResolvedTarget, CliError>) -> CliError {
        match result {
            Ok(_) => panic!("expected Err, got Ok"),
            Err(e) => e,
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_unique_location_by_name(pool: DbPool) {
        let i = sample_instance("acme").save(&pool).await.unwrap();
        sample_location("office", i.id).save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: Some("office".into()),
            tunnel: false,
            id: None,
            instance: None,
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Location(l) => {
                assert_eq!(l.name, "office");
                assert_eq!(l.instance_id, i.id);
            }
            _ => panic!("expected Location"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_unique_tunnel_by_name(pool: DbPool) {
        sample_tunnel("gateway").save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: Some("gateway".into()),
            tunnel: false,
            id: None,
            instance: None,
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Tunnel(t) => assert_eq!(t.name, "gateway"),
            _ => panic!("expected Tunnel"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_tunnel_by_name_with_tunnel_flag(pool: DbPool) {
        let i = sample_instance("acme").save(&pool).await.unwrap();
        sample_location("office", i.id).save(&pool).await.unwrap();
        sample_tunnel("office").save(&pool).await.unwrap();

        // Without --tunnel, the name clash produces an error.
        let spec = TargetSpec {
            name: Some("office".into()),
            tunnel: false,
            id: None,
            instance: None,
        };
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotFound(_)));
        assert!(err.to_string().contains("--tunnel"));

        // With --tunnel, the tunnel is resolved.
        let spec = TargetSpec {
            name: Some("office".into()),
            tunnel: true,
            id: None,
            instance: None,
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Tunnel(t) => assert_eq!(t.name, "office"),
            _ => panic!("expected Tunnel"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_cross_instance_name_collision(pool: DbPool) {
        let i1 = sample_instance("acme").save(&pool).await.unwrap();
        let i2 = sample_instance("global").save(&pool).await.unwrap();
        sample_location("office", i1.id).save(&pool).await.unwrap();
        sample_location("office", i2.id).save(&pool).await.unwrap();

        // Without --instance, ambiguous.
        let spec = TargetSpec {
            name: Some("office".into()),
            tunnel: false,
            id: None,
            instance: None,
        };
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotFound(_)));
        assert!(err.to_string().contains("--instance"));

        // With --instance, resolves to the correct one.
        let spec = TargetSpec {
            name: Some("office".into()),
            tunnel: false,
            id: None,
            instance: Some("acme".into()),
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Location(l) => assert_eq!(l.instance_id, i1.id),
            _ => panic!("expected Location"),
        }

        let spec = TargetSpec {
            name: Some("office".into()),
            tunnel: false,
            id: None,
            instance: Some("global".into()),
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Location(l) => assert_eq!(l.instance_id, i2.id),
            _ => panic!("expected Location"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_id_fast_path_location(pool: DbPool) {
        let i = sample_instance("acme").save(&pool).await.unwrap();
        let saved = sample_location("office", i.id).save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: None,
            tunnel: false,
            id: Some(saved.id),
            instance: None,
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Location(l) => assert_eq!(l.id, saved.id),
            _ => panic!("expected Location"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_id_fast_path_tunnel(pool: DbPool) {
        let t = sample_tunnel("gateway").save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: None,
            tunnel: false,
            id: Some(t.id),
            instance: None,
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Tunnel(tun) => assert_eq!(tun.id, t.id),
            _ => panic!("expected Tunnel"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_id_fast_path_tunnel_flag_skips_location(pool: DbPool) {
        let i = sample_instance("acme").save(&pool).await.unwrap();
        let saved = sample_location("office", i.id).save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: None,
            tunnel: true,
            id: Some(saved.id),
            instance: None,
        };
        // --tunnel skips Location lookup; the ID belongs to a Location so
        // no Tunnel match is found.
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotFound(_)));
        assert!(err.to_string().contains("No tunnel with id"));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_id_fast_path_tunnel_flag_with_tunnel(pool: DbPool) {
        let i = sample_instance("acme").save(&pool).await.unwrap();
        sample_location("office", i.id).save(&pool).await.unwrap();
        let t = sample_tunnel("gateway").save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: None,
            tunnel: true,
            id: Some(t.id),
            instance: None,
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Tunnel(tun) => assert_eq!(tun.id, t.id),
            _ => panic!("expected Tunnel"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_id_not_found(pool: DbPool) {
        let spec = TargetSpec {
            name: None,
            tunnel: false,
            id: Some(9999),
            instance: None,
        };
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotFound(_)));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_sole_location_no_arg(pool: DbPool) {
        let i = sample_instance("acme").save(&pool).await.unwrap();
        sample_location("office", i.id).save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: None,
            tunnel: false,
            id: None,
            instance: None,
        };
        let result = expect_ok(resolve_connect_target(&spec, &pool).await);
        match result {
            ResolvedTarget::Location(l) => assert_eq!(l.name, "office"),
            _ => panic!("expected Location"),
        }
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_no_arg_with_tunnels_only(pool: DbPool) {
        // Tunnels are ignored by the no-arg path (only locations considered).
        sample_tunnel("gateway").save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: None,
            tunnel: false,
            id: None,
            instance: None,
        };
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotEnrolled(_)));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_multiple_locations_no_arg(pool: DbPool) {
        let i = sample_instance("acme").save(&pool).await.unwrap();
        sample_location("office", i.id).save(&pool).await.unwrap();
        sample_location("home", i.id).save(&pool).await.unwrap();

        let spec = TargetSpec {
            name: None,
            tunnel: false,
            id: None,
            instance: None,
        };
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotFound(_)));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_name_not_found(pool: DbPool) {
        let spec = TargetSpec {
            name: Some("nope".into()),
            tunnel: false,
            id: None,
            instance: None,
        };
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotFound(_)));
    }

    #[sqlx::test(migrations = "../migrations")]
    async fn test_instance_not_found(pool: DbPool) {
        let spec = TargetSpec {
            name: Some("office".into()),
            tunnel: false,
            id: None,
            instance: Some("ghost".into()),
        };
        let err = expect_err(resolve_connect_target(&spec, &pool).await);
        assert!(matches!(err, CliError::NotFound(_)));
        assert!(err.to_string().contains("ghost"));
    }
}
