import './style.scss';
import clsx from 'clsx';

import type { InstanceInfo, LocationInfo } from '../../../../../shared/rust-api/types';
import {
  type CompactViewSelection,
  useAppStore,
} from '../../../../../shared/store/useAppStore';

type Props = {
  instances: InstanceInfo[];
  tunnels: LocationInfo[];
};

type SelectionItemProps = {
  label: string;
  selected: boolean;
  onClick: () => void;
};

const SelectionItem = ({ label, selected, onClick }: SelectionItemProps) => (
  <button className={clsx('item', { selected })} onClick={onClick}>
    <span>{label}</span>
  </button>
);

export const OverviewSelection = ({ instances, tunnels }: Props) => {
  const selection = useAppStore((s) => s.compactViewSelection);

  const setSelection = (value: CompactViewSelection) => {
    useAppStore.setState({ compactViewSelection: value });
  };

  const isSelected = (candidate: CompactViewSelection): boolean => {
    if (!selection) return false;
    if (candidate.kind !== selection.kind) return false;
    return candidate.data.id === selection.data.id;
  };

  return (
    <div className="overview-selection">
      {instances.length > 0 && (
        <div className="group">
          <p className="label">Instances</p>
          <div className="items">
            {instances.map((instance) => {
              const value: CompactViewSelection = { kind: 'instance', data: instance };
              return (
                <SelectionItem
                  key={instance.id}
                  label={instance.name}
                  selected={isSelected(value)}
                  onClick={() => setSelection(value)}
                />
              );
            })}
          </div>
        </div>
      )}
      {tunnels.length > 0 && (
        <div className="group">
          <p className="label">Tunnels</p>
          <div className="items">
            {tunnels.map((tunnel) => {
              const value: CompactViewSelection = { kind: 'tunnel', data: tunnel };
              return (
                <SelectionItem
                  key={tunnel.id ?? tunnel.name}
                  label={tunnel.name}
                  selected={isSelected(value)}
                  onClick={() => setSelection(value)}
                />
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
