import { useQueryClient } from '@tanstack/react-query';
import { useCallback } from 'react';

import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import type {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { useClientStore } from '../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../query';

export const StatsFilterSelect = () => {
  const filterValue = useClientStore((state) => state.statsFilter);
  const setClientStore = useClientStore((state) => state.setState);
  const queryClient = useQueryClient();

  const renderSelected = useCallback((selected: number): SelectSelectedValue => {
    return {
      key: selected,
      displayValue: `${selected}H`,
    };
  }, []);

  return (
    <Select
      renderSelected={renderSelected}
      options={selectOptions}
      selected={filterValue}
      onChangeSingle={(res) => {
        queryClient.invalidateQueries({
          queryKey: [clientQueryKeys.getLocationStats],
        });
        setClientStore({ statsFilter: res });
      }}
    />
  );
};

const selectOptions: SelectOption<number>[] = [
  {
    value: 1,
    label: '1H',
    key: 1,
  },
  {
    value: 2,
    label: '2H',
    key: 2,
  },
  {
    value: 4,
    label: '4H',
    key: 4,
  },
  {
    value: 6,
    label: '6H',
    key: 6,
  },
  {
    value: 8,
    label: '8H',
    key: 8,
  },
  {
    value: 10,
    label: '10H',
    key: 10,
  },
  {
    value: 12,
    label: '12H',
    key: 12,
  },
  {
    value: 24,
    label: '24H',
    key: 24,
  },
];
