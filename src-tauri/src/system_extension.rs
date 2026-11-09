use std::{
    sync::{LazyLock, Mutex},
    time::Duration,
};

use dispatch2::{DispatchQoS, DispatchQueue, GlobalQueueIdentifier};
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{NSObjectProtocol, ProtocolObject},
    AnyThread, DefinedClass,
};
use objc2_foundation::{NSArray, NSError, NSObject, NSProcessInfo, NSString};
use objc2_system_extensions::{
    OSSystemExtensionManager, OSSystemExtensionProperties, OSSystemExtensionReplacementAction,
    OSSystemExtensionRequest, OSSystemExtensionRequestDelegate, OSSystemExtensionRequestResult,
};
use tokio::sync::oneshot;

const PROPERTIES_TIMEOUT: Duration = Duration::from_secs(5);

// OSSystemExtensionRequest.delegate is a `weak` property, so we must keep our delegate alive
// for the duration of the activation handshake.
static DELEGATE: LazyLock<Mutex<Option<Retained<SystemExtensionDelegate>>>> =
    LazyLock::new(|| Mutex::new(None));

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "DefguardSystemExtensionDelegate"]
    struct SystemExtensionDelegate;

    unsafe impl NSObjectProtocol for SystemExtensionDelegate {}

    unsafe impl OSSystemExtensionRequestDelegate for SystemExtensionDelegate {
        /// A newer version of the extension is being installed; always replace.
        #[unsafe(method(request:actionForReplacingExtension:withExtension:))]
        fn action_for_replacing(
            &self,
            _request: &OSSystemExtensionRequest,
            _existing: &OSSystemExtensionProperties,
            _ext: &OSSystemExtensionProperties,
        ) -> OSSystemExtensionReplacementAction {
            OSSystemExtensionReplacementAction::Replace
        }

        /// The extension is waiting for user approval in System Settings > Privacy & Security.
        #[unsafe(method(requestNeedsUserApproval:))]
        fn request_needs_user_approval(&self, _request: &OSSystemExtensionRequest) {
            info!(
                "VPN system extension requires user approval — open System Settings > General > \
                Login Items & Extensions > Network Extensions to enable it."
            );
        }

        /// Activation finished (or will finish after reboot).
        #[unsafe(method(request:didFinishWithResult:))]
        fn request_did_finish(
            &self,
            _request: &OSSystemExtensionRequest,
            result: OSSystemExtensionRequestResult,
        ) {
            if result == OSSystemExtensionRequestResult::WillCompleteAfterReboot {
                info!("VPN system extension installed; activation will complete after reboot.");
            } else {
                info!("VPN system extension activated successfully.");
            }
        }

        /// Activation failed.
        #[unsafe(method(request:didFailWithError:))]
        fn request_did_fail(&self, _request: &OSSystemExtensionRequest, error: &NSError) {
            error!(
                "VPN system extension activation failed: {}",
                error.localizedDescription()
            );
        }
    }
);

impl SystemExtensionDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// Activate a system extension.
///
/// Safe to call on every launch — the OS ignores duplicate requests for extensions that are
/// already active. Callbacks arrive asynchronously on the main queue via the embedded delegate.
///
/// <https://developer.apple.com/documentation/systemextensions/installing-system-extensions-and-drivers>
pub fn activate_system_extension(bundle_id: &str) {
    let identifier = NSString::from_str(bundle_id);
    let delegate = SystemExtensionDelegate::new();

    // SAFETY: `delegate` is kept alive in DELEGATE for the duration of the async handshake, and
    // the main dispatch queue lives for the whole process.
    unsafe {
        let request = OSSystemExtensionRequest::activationRequestForExtension_queue(
            &identifier,
            DispatchQueue::main(),
        );
        request.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

        let manager = OSSystemExtensionManager::sharedManager();
        manager.submitRequest(&request);
    }

    info!("Submitted system extension activation request for {bundle_id}.");

    // Keep the delegate alive until the asynchronous callbacks are delivered.
    if let Ok(mut guard) = DELEGATE.lock() {
        *guard = Some(delegate);
    }
}

#[derive(Clone, Copy)]
pub enum SystemExtensionState {
    Enabled,
    NotEnabled,
    NotInstalled,
}

type PropertiesResult = Result<SystemExtensionState, String>;

struct PropertiesIvars {
    sender: Mutex<Option<oneshot::Sender<PropertiesResult>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "DefguardSystemExtensionPropertiesDelegate"]
    #[ivars = PropertiesIvars]
    struct PropertiesDelegate;

    unsafe impl NSObjectProtocol for PropertiesDelegate {}

    unsafe impl OSSystemExtensionRequestDelegate for PropertiesDelegate {
        // Not used by properties requests, but required by the protocol.
        #[unsafe(method(request:actionForReplacingExtension:withExtension:))]
        fn action_for_replacing(
            &self,
            _request: &OSSystemExtensionRequest,
            _existing: &OSSystemExtensionProperties,
            _ext: &OSSystemExtensionProperties,
        ) -> OSSystemExtensionReplacementAction {
            OSSystemExtensionReplacementAction::Cancel
        }

        #[unsafe(method(requestNeedsUserApproval:))]
        fn request_needs_user_approval(&self, _request: &OSSystemExtensionRequest) {}

        #[unsafe(method(request:didFinishWithResult:))]
        fn request_did_finish(
            &self,
            _request: &OSSystemExtensionRequest,
            _result: OSSystemExtensionRequestResult,
        ) {
        }

        #[unsafe(method(request:didFailWithError:))]
        fn request_did_fail(&self, _request: &OSSystemExtensionRequest, error: &NSError) {
            self.send(Err(error.localizedDescription().to_string()));
        }

        #[unsafe(method(request:foundProperties:))]
        fn request_found_properties(
            &self,
            _request: &OSSystemExtensionRequest,
            properties: &NSArray<OSSystemExtensionProperties>,
        ) {
            // Several versions may be listed, e.g. while one replaces another.
            // SAFETY: plain property getters.
            let installed: Vec<_> = properties
                .iter()
                .filter(|props| unsafe { !props.isUninstalling() })
                .collect();
            let state = if installed.is_empty() {
                SystemExtensionState::NotInstalled
            } else if installed.iter().any(|props| unsafe { props.isEnabled() }) {
                SystemExtensionState::Enabled
            } else {
                SystemExtensionState::NotEnabled
            };
            self.send(Ok(state));
        }
    }
);

impl PropertiesDelegate {
    fn new(sender: oneshot::Sender<PropertiesResult>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(PropertiesIvars {
            sender: Mutex::new(Some(sender)),
        });
        unsafe { msg_send![super(this), init] }
    }

    fn send(&self, result: PropertiesResult) {
        if let Some(sender) = self.ivars().sender.lock().ok().and_then(|mut s| s.take()) {
            let _ = sender.send(result);
        }
    }
}

/// Ask the OS for the current state of a system extension. `None` if it does not answer.
pub async fn system_extension_state(bundle_id: &str) -> Option<SystemExtensionState> {
    let (sender, receiver) = oneshot::channel();
    let delegate = PropertiesDelegate::new(sender);

    // Background queue, so the answer doesn't depend on the main thread.
    let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::QualityOfService(
        DispatchQoS::UserInitiated,
    ));
    // SAFETY: `delegate` is held until the answer arrives or the wait times out.
    unsafe {
        let identifier = NSString::from_str(bundle_id);
        let request =
            OSSystemExtensionRequest::propertiesRequestForExtension_queue(&identifier, &queue);
        request.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        OSSystemExtensionManager::sharedManager().submitRequest(&request);
    }

    let state = match tokio::time::timeout(PROPERTIES_TIMEOUT, receiver).await {
        Ok(Ok(Ok(state))) => Some(state),
        Ok(Ok(Err(err))) => {
            warn!("Failed to query the state of system extension {bundle_id}: {err}");
            None
        }
        Ok(Err(_)) | Err(_) => {
            warn!("Timed out querying the state of system extension {bundle_id}");
            None
        }
    };
    drop(delegate);

    state
}

/// System Settings pane where network extensions are allowed.
#[must_use]
pub fn settings_url() -> &'static str {
    // macOS 15 moved extension approval out of Privacy & Security.
    let major = NSProcessInfo::processInfo()
        .operatingSystemVersion()
        .majorVersion;
    if major >= 15 {
        "x-apple.systempreferences:com.apple.ExtensionsPreferences?extensionPointIdentifier=\
        com.apple.system_extension.network_extension.extension-point"
    } else {
        "x-apple.systempreferences:com.apple.preference.security?General"
    }
}

/// Re-requests activation if the extension is not installed. An unknown state counts as ready.
pub async fn vpn_extension_ready() -> bool {
    use defguard_client_core::connection::apple::PLUGIN_BUNDLE_ID;

    match system_extension_state(PLUGIN_BUNDLE_ID).await {
        None | Some(SystemExtensionState::Enabled) => true,
        Some(SystemExtensionState::NotInstalled) => {
            activate_system_extension(PLUGIN_BUNDLE_ID);
            false
        }
        Some(SystemExtensionState::NotEnabled) => false,
    }
}
