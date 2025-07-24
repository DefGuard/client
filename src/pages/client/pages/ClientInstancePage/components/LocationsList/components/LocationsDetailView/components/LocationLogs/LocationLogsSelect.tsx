import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { Select } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  type SelectOption,
  type SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../../../../../../shared/defguard-ui/components/Layout/Select/types';
import type { LogLevel } from '../../../../../../../../clientAPI/types';

type Props = {
  initSelected: LogLevel;
  onChange: (selected: LogLevel) => void;
};

export const LocationLogsSelect = ({ initSelected, onChange }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.logging.options;
  const [selected, setSelected] = useState(initSelected);

  const options = useMemo((): SelectOption<LogLevel>[] => {
    return [
      {
        key: 0,
        label: localLL.error(),
        value: 'ERROR',
      },
      {
        key: 1,
        label: localLL.info(),
        value: 'INFO',
      },
      {
        key: 2,
        label: localLL.debug(),
        value: 'DEBUG',
      },
      {
        key: 3,
        label: localLL.trace(),
        value: 'TRACE',
      },
    ];
  }, [localLL]);

  const renderSelected = useCallback(
    (value: LogLevel): SelectSelectedValue => {
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
