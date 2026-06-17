import './style.scss';
import clsx from 'clsx';
import { useAppData } from '../../../../../shared/providers/AppDataContext';
import type {
  InstanceInfo,
  LocationInfo,
  OverviewViewSelection,
} from '../../../../../shared/rust-api/types';

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
  const { viewSelection: selection, setViewSelection } = useAppData();

  const setSelection = (value: OverviewViewSelection) => {
    setViewSelection(value);
  };

  const isSelected = (candidate: OverviewViewSelection): boolean => {
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
              const value: OverviewViewSelection = { kind: 'instance', data: instance };
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
              const value: OverviewViewSelection = { kind: 'tunnel', data: tunnel };
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
