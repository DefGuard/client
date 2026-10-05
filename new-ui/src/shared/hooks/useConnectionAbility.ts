import { useMemo } from 'react';
import type { LocationInfo } from '../rust-api/types';
import {
  type ConnectionAbilityValue,
  connectionAbilityOf,
  type MfaConfigInstance,
} from '../utils/mfa';

/** For cards outside the LocationCard context, which already exposes the result. */
export const useConnectionAbility = (
  location: Pick<LocationInfo, 'connection_type' | 'mfa_steps'>,
  instance?: MfaConfigInstance,
): ConnectionAbilityValue =>
  useMemo(() => connectionAbilityOf(location, instance), [location, instance]);
