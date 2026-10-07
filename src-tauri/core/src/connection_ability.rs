//! Mirrors connectionAbilityOf in new-ui/src/shared/utils/mfa.ts, so the tray hides what the UI
//! blocks. Change both together.

use crate::{
    database::models::{
        instance::{Instance, InstanceInfo, MfaCapabilities},
        location::{LocationMfaMethod, LocationMfaStep, LocationMfaStepMethod},
    },
    mfa_config::{AUTHORIZING_METHODS, CONFIGURABLE_METHODS},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionAbility {
    /// A whole path through the steps runs on factors already on the account.
    Available,
    /// Blocked, but every blocking step offers a factor this client can set up there.
    Configurable,
    /// Blocked on a factor that cannot be set up here, or with no way to authorize the setup.
    Unavailable,
}

pub struct InstanceMfaState<'a> {
    // None when Core predates the report, the step's own flag decides then
    pub configured_methods: Option<&'a [LocationMfaMethod]>,
    pub capabilities: Option<&'a MfaCapabilities>,
    // None when Core predates the report, read as available
    pub smtp_configured: Option<bool>,
    pub openid_available: Option<bool>,
}

impl<'a, I> From<&'a Instance<I>> for InstanceMfaState<'a> {
    fn from(instance: &'a Instance<I>) -> Self {
        Self {
            configured_methods: instance
                .mfa_configured_methods
                .as_ref()
                .map(|json| json.0.as_slice()),
            capabilities: instance.mfa_capabilities.as_ref().map(|json| &json.0),
            smtp_configured: instance.smtp_configured,
            openid_available: instance.openid_available,
        }
    }
}

impl<'a, I> From<&'a InstanceInfo<I>> for InstanceMfaState<'a> {
    fn from(instance: &'a InstanceInfo<I>) -> Self {
        Self {
            configured_methods: instance.mfa_configured_methods.as_deref(),
            capabilities: instance.mfa_capabilities.as_ref(),
            smtp_configured: instance.smtp_configured,
            openid_available: instance.openid_available,
        }
    }
}

impl InstanceMfaState<'_> {
    fn is_available(&self, method: LocationMfaMethod) -> bool {
        match method {
            LocationMfaMethod::Email => self.smtp_configured != Some(false),
            LocationMfaMethod::Oidc => self.openid_available != Some(false),
            _ => true,
        }
    }

    fn is_configured(&self, entry: &LocationMfaStepMethod) -> bool {
        self.configured_methods
            .map_or(entry.configured, |methods| methods.contains(&entry.method))
    }

    fn is_usable(&self, entry: &LocationMfaStepMethod) -> bool {
        entry.method != LocationMfaMethod::Biometric
            && self.is_configured(entry)
            && self.is_available(entry.method)
    }

    fn can_set_up(&self, method: LocationMfaMethod) -> bool {
        CONFIGURABLE_METHODS.contains(&method.into())
            && self
                .capabilities
                .is_some_and(|capabilities| capabilities.can_set_up(method))
            && self.is_available(method)
    }

    /// A held factor the instance authorizes with, or the email fallback Core offers otherwise.
    fn can_authorize_config(&self) -> bool {
        let holds_authorizer = AUTHORIZING_METHODS.iter().any(|&method| {
            let method = LocationMfaMethod::from(method);
            self.capabilities
                .is_some_and(|capabilities| capabilities.can_authorize(method))
                && self
                    .configured_methods
                    .is_some_and(|methods| methods.contains(&method))
                && self.is_available(method)
        });
        holds_authorizer || self.is_available(LocationMfaMethod::Email)
    }
}

/// Steps as stored for a location, empty when MFA is off.
#[must_use]
pub fn connection_ability(
    steps: &[LocationMfaStep],
    instance: &InstanceMfaState,
) -> ConnectionAbility {
    let mut blocked_steps = steps
        .iter()
        .filter(|step| !step.methods.iter().any(|entry| instance.is_usable(entry)))
        .peekable();
    if blocked_steps.peek().is_none() {
        return ConnectionAbility::Available;
    }

    let fixable = blocked_steps.all(|step| {
        step.methods
            .iter()
            .any(|entry| instance.can_set_up(entry.method))
    });
    if fixable && instance.can_authorize_config() {
        ConnectionAbility::Configurable
    } else {
        ConnectionAbility::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(methods: &[LocationMfaMethod]) -> LocationMfaStep {
        LocationMfaStep {
            methods: methods
                .iter()
                .map(|&method| LocationMfaStepMethod {
                    method,
                    configured: false,
                })
                .collect(),
        }
    }

    fn capabilities(
        setup: &[LocationMfaMethod],
        authorize: &[LocationMfaMethod],
    ) -> MfaCapabilities {
        MfaCapabilities {
            setup_methods: setup.to_vec(),
            authorize_methods: authorize.to_vec(),
        }
    }

    fn state<'a>(
        configured: &'a [LocationMfaMethod],
        capabilities: Option<&'a MfaCapabilities>,
    ) -> InstanceMfaState<'a> {
        InstanceMfaState {
            configured_methods: Some(configured),
            capabilities,
            smtp_configured: None,
            openid_available: None,
        }
    }

    #[test]
    fn test_no_steps_is_available() {
        assert_eq!(
            connection_ability(&[], &state(&[], None)),
            ConnectionAbility::Available
        );
    }

    #[test]
    fn test_held_factor_is_available() {
        let steps = [step(&[LocationMfaMethod::Totp])];
        assert_eq!(
            connection_ability(&steps, &state(&[LocationMfaMethod::Totp], None)),
            ConnectionAbility::Available
        );
    }

    #[test]
    fn test_unreported_configured_methods_fall_back_to_step_flag() {
        let steps = [LocationMfaStep {
            methods: vec![LocationMfaStepMethod {
                method: LocationMfaMethod::Totp,
                configured: true,
            }],
        }];
        let instance = InstanceMfaState {
            configured_methods: None,
            ..state(&[], None)
        };
        assert_eq!(
            connection_ability(&steps, &instance),
            ConnectionAbility::Available
        );
    }

    #[test]
    fn test_settable_blocking_factor_is_configurable() {
        let steps = [step(&[LocationMfaMethod::Email])];
        let caps = capabilities(&[LocationMfaMethod::Email], &[]);
        assert_eq!(
            connection_ability(&steps, &state(&[], Some(&caps))),
            ConnectionAbility::Configurable
        );
    }

    #[test]
    fn test_unsettable_blocking_factor_is_unavailable() {
        let steps = [step(&[LocationMfaMethod::Email])];
        let caps = capabilities(&[LocationMfaMethod::Totp], &[]);
        assert_eq!(
            connection_ability(&steps, &state(&[], Some(&caps))),
            ConnectionAbility::Unavailable
        );
        assert_eq!(
            connection_ability(&steps, &state(&[], None)),
            ConnectionAbility::Unavailable
        );
    }

    #[test]
    fn test_email_without_smtp_is_unavailable() {
        let steps = [step(&[LocationMfaMethod::Email])];
        let caps = capabilities(&[LocationMfaMethod::Email], &[LocationMfaMethod::Email]);
        let instance = InstanceMfaState {
            smtp_configured: Some(false),
            ..state(&[LocationMfaMethod::Email], Some(&caps))
        };
        assert_eq!(
            connection_ability(&steps, &instance),
            ConnectionAbility::Unavailable
        );
    }

    #[test]
    fn test_openid_without_provider_is_unavailable() {
        let steps = [step(&[LocationMfaMethod::Oidc])];
        let caps = capabilities(&[LocationMfaMethod::Totp], &[LocationMfaMethod::Totp]);
        let instance = InstanceMfaState {
            openid_available: Some(false),
            ..state(&[LocationMfaMethod::Oidc], Some(&caps))
        };
        assert_eq!(
            connection_ability(&steps, &instance),
            ConnectionAbility::Unavailable
        );
    }

    #[test]
    fn test_nothing_to_authorize_setup_is_unavailable() {
        let steps = [step(&[LocationMfaMethod::Totp])];
        let caps = capabilities(&[LocationMfaMethod::Totp], &[LocationMfaMethod::Totp]);
        let instance = InstanceMfaState {
            smtp_configured: Some(false),
            ..state(&[], Some(&caps))
        };
        assert_eq!(
            connection_ability(&steps, &instance),
            ConnectionAbility::Unavailable
        );
    }

    #[test]
    fn test_held_authorizer_without_smtp_is_configurable() {
        let steps = [step(&[LocationMfaMethod::Fido2])];
        let caps = capabilities(&[LocationMfaMethod::Fido2], &[LocationMfaMethod::Totp]);
        let instance = InstanceMfaState {
            smtp_configured: Some(false),
            ..state(&[LocationMfaMethod::Totp], Some(&caps))
        };
        assert_eq!(
            connection_ability(&steps, &instance),
            ConnectionAbility::Configurable
        );
    }

    #[test]
    fn test_biometric_is_not_desktop_usable() {
        let steps = [step(&[LocationMfaMethod::Biometric])];
        assert_eq!(
            connection_ability(&steps, &state(&[LocationMfaMethod::Biometric], None)),
            ConnectionAbility::Unavailable
        );
    }
}
