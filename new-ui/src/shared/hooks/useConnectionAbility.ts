import { useMemo } from 'react';
import type { InstanceInfo, LocationInfo } from '../rust-api/types';
import { type ConnectionAbilityValue, connectionAbilityOf } from '../utils/mfa';

/** `connectionAbilityOf` memoized on its inputs. The LocationCard context exposes the result
 *  as `connectionAbility`; cards outside that context call this directly. */
export const useConnectionAbility = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
  instance?: Pick<InstanceInfo, 'mfa_configured_methods'>,
): ConnectionAbilityValue =>
  useMemo(() => connectionAbilityOf(location, instance), [location, instance]);
