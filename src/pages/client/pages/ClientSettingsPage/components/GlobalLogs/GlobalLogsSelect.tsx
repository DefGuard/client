import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { GlobalLogLevel } from '../../../../clientAPI/types';

type Props = {
  initSelected: GlobalLogLevel;
  onChange: (selected: GlobalLogLevel) => void;
};

export const GlobalLogsSelect = ({ initSelected, onChange }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.settingsPage.tabs.global.logging.options;
  const [selected, setSelected] = useState(initSelected);

  const options = useMemo((): SelectOption<GlobalLogLevel>[] => {
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
    ];
  }, [localLL]);

  const renderSelected = useCallback(
    (value: GlobalLogLevel): SelectSelectedValue => {
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
