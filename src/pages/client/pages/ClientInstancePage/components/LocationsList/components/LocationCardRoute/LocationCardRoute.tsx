import { useMemo } from 'react';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { Toggle } from '../../../../../../../../shared/defguard-ui/components/Layout/Toggle/Toggle';
import { ToggleOption } from '../../../../../../../../shared/defguard-ui/components/Layout/Toggle/types';
import { DefguardLocation, RouteOption } from '../../../../../../types';

type Props = {
  location?: DefguardLocation;
  onChange: (v: number) => void;
  selected: number;
};

export const LocationCardRoute = ({ location, onChange, selected }: Props) => {
  const { LL } = useI18nContext();
  const toggleOptions = useMemo(() => {
    const res: ToggleOption<number>[] = [
      {
        text: LL.pages.client.pages.instancePage.controls.predefinedTraffic(),
        value: RouteOption.PREDEFINED_TRAFFIC,
      },
      {
        text: LL.pages.client.pages.instancePage.controls.allTraffic(),
        value: RouteOption.ALL_TRAFFIC,
      },
    ];
    return res;
  }, [LL.pages]);

  return (
    <Toggle
      options={toggleOptions}
      selected={selected}
      disabled={location?.active}
      onChange={(v) => onChange(v)}
    />
  );
};
