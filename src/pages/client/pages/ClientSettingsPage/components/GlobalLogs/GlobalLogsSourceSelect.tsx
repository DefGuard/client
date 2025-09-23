import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  type SelectOption,
  type SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import type { LogSource } from '../../../../clientAPI/types';

type Props = {
  initSelected: LogSource;
  onChange: (selected: LogSource) => void;
};

export const GlobalLogsSourceSelect = ({ initSelected, onChange }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.globalLogs.logSources;
  const [selected, setSelected] = useState(initSelected);

  const options = useMemo((): SelectOption<LogSource>[] => {
    return [
      {
        key: 0,
        label: localLL.all(),
        value: 'All',
      },
      {
        key: 1,
        label: localLL.client(),
        value: 'Client',
      },
      {
        key: 2,
        label: localLL.service(),
        value: 'Service',
      },
    ];
  }, [localLL]);

  const renderSelected = useCallback(
    (value: LogSource): SelectSelectedValue => {
      const option = options.find((o) => o.value === value);
      if (option) {
        return {
          key: option.key,
          displayValue: option.label,
        };
      }
      return {
        key: 0,
        displayValue: '',
      };
    },
    [options],
  );

  return (
    <Select
      sizeVariant={SelectSizeVariant.SMALL}
      selected={selected}
      renderSelected={renderSelected}
      options={options}
      onChangeSingle={(res) => {
        setSelected(res);
        onChange(res);
      }}
    />
  );
};
