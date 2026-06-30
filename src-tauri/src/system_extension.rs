use std::sync::{LazyLock, Mutex};

use objc2::{define_class, msg_send, rc::Retained, runtime::AnyObject, AnyThread};
use objc2_foundation::{NSError, NSObject, NSString};

// OSSystemExtensionRequest.delegate is a `weak` property, so we must keep our delegate alive
// for the duration of the activation handshake.
static DELEGATE: LazyLock<Mutex<Option<Retained<SystemExtensionDelegate>>>> =
    LazyLock::new(|| Mutex::new(None));

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "DefguardSystemExtensionDelegate"]
    struct SystemExtensionDelegate;

    impl SystemExtensionDelegate {
        /// The extension is waiting for user approval in System Settings > Privacy & Security.
        #[unsafe(method(requestNeedsUserApproval:))]
        fn request_needs_user_approval(&self, _request: &AnyObject) {
            info!(
                "VPN system extension requires user approval — open \
                System Settings > Privacy & Security to enable it."
            );
        }

        /// Activation finished (or will finish after reboot).
        #[unsafe(method(request:didFinishWithResult:))]
        fn request_did_finish(&self, _request: &AnyObject, result: isize) {
            // OSSystemExtensionResult::WillCompleteAfterReboot == 1
            if result == 1 {
                info!("VPN system extension installed; activation will complete after reboot.");
            } else {
                info!("VPN system extension activated successfully.");
            }
        }

        /// Activation failed.
        #[unsafe(method(request:didFailWithError:))]
        fn request_did_fail(&self, _request: &AnyObject, error: &NSError) {
            error!(
                "VPN system extension activation failed: {}",
                error.localizedDescription()
            );
        }

        /// A newer version of the extension is being installed; always replace.
        #[unsafe(method(request:actionForReplacingExtension:withExtension:))]
        fn action_for_replacing(
            &self,
            _request: &AnyObject,
            _old: &AnyObject,
            _new: &AnyObject,
        ) -> isize {
            1 // OSSystemExtensionReplacementAction::Replace
        }
    }
);

impl SystemExtensionDelegate {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}

/// Submit a system extension activation request for `bundle_id` to
/// `OSSystemExtensionManager`. Safe to call on every launch — the OS ignores duplicate
/// requests for extensions that are already active. Callbacks arrive asynchronously on the
/// main queue via the embedded delegate.
///
/// Silently skips when `OSSystemExtensionRequest` is absent from the ObjC runtime, which
/// happens for App Store builds (no `packet-tunnel-provider-systemextension` entitlement)
/// or when `SystemExtensions.framework` is not linked.
pub fn activate_system_extension(bundle_id: &str) {
    use objc2::runtime::AnyClass;

    let Some(req_class) = AnyClass::get(c"OSSystemExtensionRequest") else {
        debug!(
            "OSSystemExtensionRequest not found in ObjC runtime — \
            skipping system extension activation."
        );
        return;
    };
    let Some(mgr_class) = AnyClass::get(c"OSSystemExtensionManager") else {
        warn!("OSSystemExtensionManager not found — skipping system extension activation.");
        return;
    };

    let ext_id = NSString::from_str(bundle_id);
    let delegate = SystemExtensionDelegate::new();

    unsafe {
        // Passing nil for the queue uses the main queue (documented behaviour).
        let queue: *mut std::ffi::c_void = std::ptr::null_mut();

        let request: Option<Retained<AnyObject>> = msg_send![
            req_class,
            activationRequestForExtensionWithIdentifier: &*ext_id,
            queue: queue
        ];
        let Some(request) = request else {
            error!("Failed to create system extension activation request for {bundle_id}.");
            return;
        };

        // Set our delegate (weak reference on the request side — strong ref kept in DELEGATE).
        let delegate_ns: &NSObject = &*delegate;
        let _: () = msg_send![&*request, setDelegate: delegate_ns];

        let manager: Retained<AnyObject> = msg_send![mgr_class, sharedManager];
        let _: () = msg_send![&*manager, submitRequest: &*request];
    }

    info!("Submitted system extension activation request for {bundle_id}.");

    // Keep the delegate alive until the asynchronous callbacks are delivered.
    if let Ok(mut guard) = DELEGATE.lock() {
        *guard = Some(delegate);
    }
}
