import './style.scss';

import { error } from '@tauri-apps/plugin-log';
import { useMemo } from 'react';
import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { Toggle } from '../../../../../../../../shared/defguard-ui/components/Layout/Toggle/Toggle';
import type { ToggleOption } from '../../../../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import {
  ClientConnectionType,
  ClientTrafficPolicy,
  type CommonWireguardFields,
  type DefguardInstance,
} from '../../../../../../types';

type Props = {
  location?: CommonWireguardFields;
  selectedDefguardInstance?: DefguardInstance;
};
const { updateLocationRouting } = clientApi;

export const LocationCardRoute = ({ location, selectedDefguardInstance }: Props) => {
  const handleChange = async (value: boolean) => {
    try {
      if (location?.connection_type) {
        await updateLocationRouting({
          locationId: location?.id,
          connectionType: location.connection_type,
          routeAllTraffic: value,
        });
      }
    } catch (e) {
      error(`Error handling routing: ${e}`);
      console.error(e);
    }
  };

  const { LL } = useI18nContext();

  const toggleOptions = useMemo(() => {
    const res: ToggleOption<boolean>[] = [
      {
        text: LL.pages.client.pages.instancePage.controls.traffic.predefinedTraffic(),
        value: false,
        disabled:
          selectedDefguardInstance?.client_traffic_policy ===
          ClientTrafficPolicy.FORCE_ALL_TRAFFIC,
      },
      {
        text: LL.pages.client.pages.instancePage.controls.traffic.allTraffic(),
        value: true,
        disabled:
          selectedDefguardInstance?.client_traffic_policy ===
          ClientTrafficPolicy.DISABLE_ALL_TRAFFIC,
      },
    ];
    return res;
  }, [LL.pages, selectedDefguardInstance?.client_traffic_policy]);

  const selected = useMemo((): boolean => {
    if (!selectedDefguardInstance || !location) return false;

    // tunnel
    if (location.connection_type === ClientConnectionType.TUNNEL)
      return location.route_all_traffic;

    // Defguard location
    switch (selectedDefguardInstance.client_traffic_policy) {
      case ClientTrafficPolicy.DISABLE_ALL_TRAFFIC:
        return false;
      case ClientTrafficPolicy.FORCE_ALL_TRAFFIC:
        return true;
      case ClientTrafficPolicy.NONE:
        return location.route_all_traffic ?? false;
    }
  }, [location, selectedDefguardInstance]);

  return (
    <Toggle
      className="location-traffic-toggle"
      options={toggleOptions}
      selected={selected}
      disabled={location?.active}
      onChange={(v) => {
        if (!location?.active) {
          handleChange(v);
        }
      }}
    />
  );
};
