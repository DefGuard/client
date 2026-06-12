import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { Select } from '../../../../shared/components/Select/Select';
import type {
  SelectOption,
  SelectOptionGroup,
} from '../../../../shared/components/Select/types';
import {
  getInstancesQueryOptions,
  getTunnelsQueryOptions,
} from '../../../../shared/rust-api/query';
import {
  type CompactViewSelection,
  useAppStore,
} from '../../../../shared/store/useAppStore';
import { isPresent } from '../../../../shared/utils/isPresent';

export const InstanceSwitcher = () => {
  const selectedInstance = useAppStore((s) => s.compactViewSelection);

  const { data: tunnels } = useQuery(getTunnelsQueryOptions);
  const { data: instances } = useQuery(getInstancesQueryOptions);

  const groups = useMemo((): readonly SelectOptionGroup<CompactViewSelection>[] => {
    if (!isPresent(instances) || !isPresent(tunnels)) return [];

    const instanceGroup: SelectOptionGroup<CompactViewSelection> = {
      key: 'instances',
      label: 'Instances',
      options: instances.map((instance) => ({
        key: instance.id,
        label: instance.name,
        value: { kind: 'instance', data: instance },
      })),
    };

    const tunnelGroup: SelectOptionGroup<CompactViewSelection> = {
      key: 'tunnels',
      label: 'Tunnels',
      options: tunnels.map((tunnel) => ({
        key: tunnel.id ?? tunnel.name,
        label: tunnel.name,
        value: { kind: 'tunnel', data: tunnel },
      })),
    };

    return [instanceGroup, tunnelGroup];
  }, [instances, tunnels]);

  const totalOptions = useMemo(
    () => groups.reduce((acc, g) => acc + g.options.length, 0),
    [groups],
  );

  const selectedOption = useMemo((): SelectOption<CompactViewSelection> | undefined => {
    if (!isPresent(selectedInstance)) return undefined;
    for (const group of groups) {
      const found = group.options.find((o) => {
        if (selectedInstance.kind === 'instance' && o.value.kind === 'instance') {
          return o.value.data.id === selectedInstance.data.id;
        }
        if (selectedInstance.kind === 'tunnel' && o.value.kind === 'tunnel') {
          return o.value.data.id === selectedInstance.data.id;
        }
        return false;
      });
      if (found) return found;
    }
    return undefined;
  }, [selectedInstance, groups]);

  if (!isPresent(instances) || !isPresent(tunnels)) return null;
  if (totalOptions <= 1) return null;

  return (
    <Select
      groups={groups}
      value={selectedOption as never}
      onChange={(option) => {
        useAppStore.setState({ compactViewSelection: option.value });
      }}
    />
  );
};
