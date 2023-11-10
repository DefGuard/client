import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectProps,
  SelectSelectedValue,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { useClientStore } from '../../../../hooks/useClientStore';
import { ClientView } from '../../../../types';

export const StatsLayoutSelect = () => {
  const { LL } = useI18nContext();

  const selectedView = useClientStore((state) => state.selectedView);

  const setClientStore = useClientStore((state) => state.setState);

  const localLL = LL.pages.client.pages.instancePage;

  const options = useMemo(
    (): SelectOption<ClientView>[] => [
      { key: 0, value: ClientView.GRID, label: localLL.header.filters.views.grid() },
      {
        key: 1,
        value: ClientView.DETAIL,
        label: localLL.header.filters.views.detail(),
      },
    ],
    [localLL.header.filters.views],
  );

  const renderSelected: SelectProps<ClientView>['renderSelected'] = useCallback(
    (value): SelectSelectedValue => {
      const selected = options.find((o) => o.value === value);
      if (selected) {
        return {
          key: selected.key,
          displayValue: selected.label,
        };
      }
      return {
        key: 'ERROR',
        displayValue: 'None',
      };
    },
    [options],
  );

  return (
    <Select<ClientView>
      options={options}
      renderSelected={renderSelected}
      selected={selectedView}
      onChangeSingle={(val) => setClientStore({ selectedView: val })}
    />
  );
};
