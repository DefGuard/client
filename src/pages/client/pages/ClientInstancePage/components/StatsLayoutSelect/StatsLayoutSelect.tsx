import { useMutation } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectProps,
  SelectSelectedValue,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { ClientView } from '../../../../clientAPI/types';
import { useClientStore } from '../../../../hooks/useClientStore';
import { CommonWireguardFields } from '../../../../types';

interface StatsLayoutSelect {
  locations: CommonWireguardFields[] | undefined;
}

export const StatsLayoutSelect = ({ locations }: StatsLayoutSelect) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage;
  const settings = useClientStore((state) => state.settings);
  const updateClientSettings = useClientStore((state) => state.updateSettings);
  const { mutate, isPending } = useMutation({
    mutationFn: updateClientSettings,
  });

  const options = useMemo(
    (): SelectOption<ClientView>[] => [
      { key: 0, value: 'grid', label: localLL.header.filters.views.grid() },
      {
        key: 1,
        value: 'detail',
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
      } else if (selected == null && locations != undefined) {
        if (locations.length == 1) {
          return {
            key: 'detail',
            displayValue: localLL.header.filters.views.detail(),
          };
        }
        return {
          key: 'grid',
          displayValue: localLL.header.filters.views.grid(),
        };
      }
      return {
        key: 'grid',
        displayValue: localLL.header.filters.views.grid(),
      };
    },
    [options, locations, localLL.header.filters.views],
  );

  return (
    <Select<ClientView>
      options={options}
      renderSelected={renderSelected}
      selected={settings.selected_view}
      onChangeSingle={(view) => mutate({ selected_view: view })}
      loading={isPending}
    />
  );
};
