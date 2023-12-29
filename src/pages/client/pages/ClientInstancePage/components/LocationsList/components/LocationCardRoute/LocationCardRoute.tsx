import './style.scss';

import { useMemo } from 'react';
import { error } from 'tauri-plugin-log-api';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { Toggle } from '../../../../../../../../shared/defguard-ui/components/Layout/Toggle/Toggle';
import { ToggleOption } from '../../../../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { CommonWireguardFields } from '../../../../../../types';

type Props = {
  location?: CommonWireguardFields;
};
const { updateLocationRouting } = clientApi;

export const LocationCardRoute = ({ location }: Props) => {
  const handleChange = async (value: boolean) => {
    try {
      if (location) {
        await updateLocationRouting({
          locationId: location?.id,
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
    const res: ToggleOption<number>[] = [
      {
        text: LL.pages.client.pages.instancePage.controls.traffic.predefinedTraffic(),
        value: 0,
      },
      {
        text: LL.pages.client.pages.instancePage.controls.traffic.allTraffic(),
        value: 1,
      },
    ];
    return res;
  }, [LL.pages]);

  return (
    <Toggle
      className="location-traffic-toggle"
      options={toggleOptions}
      selected={Number(location?.route_all_traffic)}
      disabled={location?.active}
      onChange={(v) => {
        if (!location?.active) {
          handleChange(Boolean(v));
        }
      }}
    />
  );
};
