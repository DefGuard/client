use std::sync::{LazyLock, Mutex};

use dispatch2::DispatchQueue;
use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{NSObjectProtocol, ProtocolObject},
    AnyThread,
};
use objc2_foundation::{NSError, NSObject, NSString};
use objc2_system_extensions::{
    OSSystemExtensionManager, OSSystemExtensionProperties, OSSystemExtensionReplacementAction,
    OSSystemExtensionRequest, OSSystemExtensionRequestDelegate, OSSystemExtensionRequestResult,
};

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
