use defguard_client_proto::defguard::{
    client::v1::{AttestDeviceIdentityRequest, CreateDeviceIdentityResponse},
    device_identity::v2::AttestationChallengeSolution,
};
use tonic::Status;

#[cfg(target_os = "linux")]
mod linux;

#[derive(Default)]
pub(crate) struct DeviceIdentityService {
    #[cfg(target_os = "linux")]
    operation: std::sync::Arc<tokio::sync::Mutex<()>>,
}

impl DeviceIdentityService {
    #[cfg(target_os = "linux")]
    pub(crate) async fn create(&self) -> Result<CreateDeviceIdentityResponse, Status> {
        self.run(linux::create()).await
    }

    #[cfg(target_os = "linux")]
    pub(crate) async fn attest(
        &self,
        request: AttestDeviceIdentityRequest,
    ) -> Result<AttestationChallengeSolution, Status> {
        let request = linux::prepare(request)?;
        self.run(linux::attest(request)).await
    }

    #[cfg(target_os = "linux")]
    async fn run<T, F>(&self, operation: F) -> Result<T, Status>
    where
        T: Send + 'static,
        F: std::future::Future<Output = Result<T, Status>> + Send + 'static,
    {
        let guard = std::sync::Arc::clone(&self.operation).lock_owned().await;
        // Cancelling an RPC does not stop spawn_blocking, so its task must retain the TPM lock.
        tokio::spawn(async move {
            let _operation = guard;
            operation.await
        })
        .await
        .map_err(|_| Status::internal("Device identity operation task failed"))?
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) async fn create(&self) -> Result<CreateDeviceIdentityResponse, Status> {
        Err(Status::unimplemented(
            "Device identity is only supported on Linux",
        ))
    }

    #[cfg(not(target_os = "linux"))]
    pub(crate) async fn attest(
        &self,
        _request: AttestDeviceIdentityRequest,
    ) -> Result<AttestationChallengeSolution, Status> {
        Err(Status::unimplemented(
            "Device identity is only supported on Linux",
        ))
    }
}
