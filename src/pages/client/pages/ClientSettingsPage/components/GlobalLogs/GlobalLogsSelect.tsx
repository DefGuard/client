import { useCallback, useMemo, useState } from 'react';
import { LogLevel } from '../../../../clientAPI/types';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';

type Props = {
  initSelected: LogLevel;
  onChange: (selected: LogLevel) => void;
};

export const GlobalLogsSelect = ({ initSelected, onChange }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.logging.options;
  const [selected, setSelected] = useState(initSelected);

  const options = useMemo((): SelectOption<LogLevel>[] => {
    return [
      {
        key: 0,
        label: localLL.error(),
        value: 'error',
      },
      {
        key: 1,
        label: localLL.info(),
        value: 'info',
      },
      {
        key: 2,
        label: localLL.debug(),
        value: 'debug',
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
