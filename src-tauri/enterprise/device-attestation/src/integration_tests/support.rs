use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use defguard_client_proto::defguard::{
    client::v1::{
        self as ipc,
        desktop_daemon_service_client::DesktopDaemonServiceClient,
        desktop_daemon_service_server::{DesktopDaemonService, DesktopDaemonServiceServer},
    },
    client_types::{self as http, AttestationFinishResponse, AttestationStartResponse},
    device_identity::v2::{self as identity, attestation_challenge_solution::Result as Proof},
    enterprise::posture::v2::DevicePostureData,
};
use hyper_util::rt::TokioIo;
use sqlx::SqlitePool;
use tempfile::TempDir;
use tokio::{
    net::{UnixListener, UnixStream},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{
    transport::{Channel, Endpoint, Server},
    Request, Response, Status,
};
use tower::service_fn;
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

use defguard_client_core::{
    database::models::{
        device_identity::DeviceIdentity, instance::Instance, wireguard_keys::WireguardKeys, Id,
        NoId,
    },
    enrollment::{enrollment_create_device, enrollment_start, EnrollmentSession},
};

pub(super) const DEVICE_ID: Id = 701;
pub(super) const SESSION_ID: &str = "attestation-session";

pub(super) fn material(seed: u8) -> ipc::DeviceIdentityMaterial {
    ipc::DeviceIdentityMaterial {
        backend: Some(ipc::device_identity_material::Backend::Tpm(
            ipc::TpmIdentityMaterial {
                ek: Some(ipc::TpmEndorsementKeyMaterial {
                    template: vec![seed, 1],
                    public: vec![seed, 2],
                    certificate_chain_der: vec![vec![seed, 3]],
                }),
                srk_template: vec![seed, 4],
                ak: Some(ipc::TpmKeyBlobs {
                    public: vec![seed, 5],
                    private: vec![seed, 6],
                }),
                identity_key: Some(ipc::TpmKeyBlobs {
                    public: vec![seed, 7],
                    private: vec![seed, 8],
                }),
            },
        )),
    }
}

fn public() -> identity::AttestationIdentity {
    identity::AttestationIdentity {
        backend: Some(identity::attestation_identity::Backend::Tpm(
            identity::TpmAttestationIdentity {
                ek_public: vec![0, 255, 2],
                ak_public: vec![0, 254, 5],
                ek_certificate_chain_der: vec![vec![0, 253, 3]],
            },
        )),
    }
}

pub(super) fn challenge() -> identity::AttestationChallenge {
    identity::AttestationChallenge {
        session_id: SESSION_ID.into(),
        data: vec![0, 255, 10],
        expires_at: 1_900_000_000,
        challenge: Some(identity::attestation_challenge::Challenge::Tpm(
            identity::CredentialChallenge {
                credential_blob: vec![0, 254, 11],
                encrypted_secret: vec![0, 253, 12],
            },
        )),
    }
}

pub(super) fn solution() -> identity::AttestationChallengeSolution {
    identity::AttestationChallengeSolution {
        session_id: SESSION_ID.into(),
        result: Some(Proof::Tpm(identity::TpmAttestationResponse {
            activation_secret: vec![0, 252, 13],
            evidence: Some(identity::TpmEvidence {
                ek_public: vec![0, 255, 2],
                ek_certificate_chain_der: vec![vec![0, 253, 3]],
                ak_public: vec![0, 254, 5],
                identity_public: vec![0, 251, 14],
                attest: vec![0, 250, 15],
                signature: Some(identity::Signature {
                    algorithm: identity::SignatureAlgorithm::EcdsaSha256.into(),
                    signature: vec![0, 249, 16],
                }),
            }),
        })),
    }
}

pub(super) fn verified() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(AttestationFinishResponse {
        identity_state: Some(identity::IdentityState {
            identity_id: "core-identity".into(),
            attestation_status: identity::AttestationStatus::Attested.into(),
            key_fingerprint: vec![1; 32],
            ..Default::default()
        }),
    })
}

pub(super) struct Registration {
    pub proxy: MockServer,
    pub session: EnrollmentSession,
    pub instance: Instance<Id>,
}

impl Registration {
    pub async fn new(pool: &SqlitePool, uuid: &str, enrolled: bool) -> Self {
        let proxy = MockServer::start().await;
        let info = http::InstanceInfo {
            id: uuid.into(),
            username: "alice".into(),
            name: uuid.into(),
            url: "https://core.example".into(),
            proxy_url: proxy.uri(),
            enterprise_enabled: true,
            ..Default::default()
        };
        Mock::given(method("POST"))
            .and(path("/api/v1/enrollment/start"))
            .and(body_json(serde_json::json!({"token": "enrollment-token"})))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(
                        "Set-Cookie",
                        "defguard_proxy=private-cookie; Path=/api/v1/enrollment",
                    )
                    .set_body_json(http::EnrollmentStartResponse {
                        instance: Some(info.clone()),
                        user: Some(http::InitialUserInfo {
                            enrolled,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
            )
            .expect(1)
            .mount(&proxy)
            .await;
        let (session, _) =
            enrollment_start(proxy.uri().parse().unwrap(), "enrollment-token".into())
                .await
                .unwrap();
        Mock::given(method("POST"))
            .and(path("/api/v1/enrollment/create_device"))
            .and(header("Cookie", "defguard_proxy=private-cookie"))
            .and(body_json(
                serde_json::json!({"name": "laptop", "pubkey": "wireguard-public"}),
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(http::DeviceConfigResponse {
                    device: Some(http::Device {
                        id: DEVICE_ID,
                        pubkey: "wireguard-public".into(),
                        ..Default::default()
                    }),
                    instance: Some(info),
                    ..Default::default()
                }),
            )
            .expect(1)
            .mount(&proxy)
            .await;
        let config =
            enrollment_create_device(session.clone(), "laptop".into(), "wireguard-public".into())
                .await
                .unwrap();
        let mut transaction = pool.begin().await.unwrap();
        let instance = Instance::<NoId>::from(config.instance.unwrap())
            .save(&mut *transaction)
            .await
            .unwrap();
        WireguardKeys::new(
            instance.id,
            config.device.unwrap().pubkey,
            "wireguard-private".into(),
        )
        .save(&mut *transaction)
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        Self {
            proxy,
            session,
            instance,
        }
    }

    pub async fn expect_attestation(
        &self,
        start: ResponseTemplate,
        proof: identity::AttestationChallengeSolution,
        finish: ResponseTemplate,
    ) {
        self.expect_start(start).await;
        Mock::given(method("POST"))
            .and(path("/api/v1/enrollment/attestation/finish"))
            .and(header("Cookie", "defguard_proxy=private-cookie"))
            .and(body_json(
                serde_json::json!({"device_id": DEVICE_ID, "solution": proof}),
            ))
            .respond_with(finish)
            .expect(1)
            .mount(&self.proxy)
            .await;
    }

    pub async fn expect_start(&self, start: ResponseTemplate) {
        Mock::given(method("POST"))
            .and(path("/api/v1/enrollment/attestation/start"))
            .and(header("Cookie", "defguard_proxy=private-cookie"))
            .and(body_json(
                serde_json::json!({"device_id": DEVICE_ID, "attestation_identity": public()}),
            ))
            .respond_with(start)
            .expect(1)
            .mount(&self.proxy)
            .await;
    }

    pub async fn assert_no_attestation(&self) {
        assert!(self
            .proxy
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| !request.url.path().contains("/attestation/")));
    }

    pub async fn assert_registration_retained(&self, pool: &SqlitePool) {
        assert!(Instance::find_by_id(pool, self.instance.id)
            .await
            .unwrap()
            .is_some());
        let keys = WireguardKeys::find_by_instance_id(pool, self.instance.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(keys.pubkey, "wireguard-public");
        assert_eq!(keys.prvkey, "wireguard-private");
    }
}

pub(super) fn start_response() -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(AttestationStartResponse {
        challenge: Some(challenge()),
    })
}

struct MockDaemon {
    pool: SqlitePool,
    instance_id: Id,
    seed: u8,
    creation: Result<(), Status>,
    proof: Result<identity::AttestationChallengeSolution, Status>,
    creates: Arc<AtomicUsize>,
    solves: Arc<AtomicUsize>,
}

pub(super) struct Daemon {
    pub client: DesktopDaemonServiceClient<Channel>,
    pub creates: Arc<AtomicUsize>,
    pub solves: Arc<AtomicUsize>,
    task: JoinHandle<()>,
    _directory: TempDir,
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Daemon {
    pub async fn new(
        pool: &SqlitePool,
        instance_id: Id,
        seed: u8,
        creation: Result<(), Status>,
        proof: Result<identity::AttestationChallengeSolution, Status>,
    ) -> Self {
        let directory = tempfile::tempdir_in("/tmp").unwrap();
        let socket = directory.path().join("daemon.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let creates = Arc::new(AtomicUsize::new(0));
        let solves = Arc::new(AtomicUsize::new(0));
        let daemon = MockDaemon {
            pool: pool.clone(),
            instance_id,
            seed,
            creation,
            proof,
            creates: creates.clone(),
            solves: solves.clone(),
        };
        let task = tokio::spawn(async move {
            Server::builder()
                .add_service(DesktopDaemonServiceServer::new(daemon))
                .serve_with_incoming(UnixListenerStream::new(listener))
                .await
                .unwrap();
        });
        let channel = Endpoint::from_static("http://localhost")
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(5))
            .connect_with_connector(service_fn(move |_: tonic::transport::Uri| {
                let socket = socket.clone();
                async move { UnixStream::connect(socket).await.map(TokioIo::new) }
            }))
            .await
            .unwrap();
        Self {
            client: DesktopDaemonServiceClient::new(channel),
            creates,
            solves,
            task,
            _directory: directory,
        }
    }
    pub fn counts(&self) -> (usize, usize) {
        (
            self.creates.load(Ordering::SeqCst),
            self.solves.load(Ordering::SeqCst),
        )
    }
}

#[tonic::async_trait]
impl DesktopDaemonService for MockDaemon {
    type ReadInterfaceDataStream = futures_util::stream::Empty<Result<ipc::InterfaceData, Status>>;
    async fn create_device_identity(
        &self,
        request: Request<()>,
    ) -> Result<Response<ipc::CreateDeviceIdentityResponse>, Status> {
        assert!(!request.metadata().contains_key("cookie"));
        self.creates.fetch_add(1, Ordering::SeqCst);
        self.creation.clone()?;
        Ok(Response::new(ipc::CreateDeviceIdentityResponse {
            identity: Some(material(self.seed)),
            attestation_identity: Some(public()),
        }))
    }
    async fn attest_device_identity(
        &self,
        request: Request<ipc::AttestDeviceIdentityRequest>,
    ) -> Result<Response<identity::AttestationChallengeSolution>, Status> {
        assert!(!request.metadata().contains_key("cookie"));
        self.solves.fetch_add(1, Ordering::SeqCst);
        let request = request.into_inner();
        let saved = DeviceIdentity::find_by_instance_id(&self.pool, self.instance_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.core_device_id, DEVICE_ID);
        assert!(saved.identity_id.is_none());
        assert!(saved.material == material(self.seed));
        assert!(request.identity == Some(saved.material));
        assert_eq!(request.challenge, Some(challenge()));
        self.proof.clone().map(Response::new)
    }
    async fn create_interface(
        &self,
        _: Request<ipc::CreateInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn remove_interface(
        &self,
        _: Request<ipc::RemoveInterfaceRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn read_interface_data(
        &self,
        _: Request<ipc::ReadInterfaceDataRequest>,
    ) -> Result<Response<Self::ReadInterfaceDataStream>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn save_service_locations(
        &self,
        _: Request<ipc::SaveServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn delete_service_locations(
        &self,
        _: Request<ipc::DeleteServiceLocationsRequest>,
    ) -> Result<Response<()>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn get_posture_data(
        &self,
        _: Request<()>,
    ) -> Result<Response<DevicePostureData>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn list_interfaces(
        &self,
        _: Request<()>,
    ) -> Result<Response<ipc::ListInterfacesResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
    async fn start_update(
        &self,
        _: Request<()>,
    ) -> Result<Response<ipc::StartUpdateResponse>, Status> {
        Err(Status::unimplemented("unused"))
    }
}
