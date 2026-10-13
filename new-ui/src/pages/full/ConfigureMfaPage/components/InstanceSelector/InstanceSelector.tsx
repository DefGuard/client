import { IconKind } from '../../../../../shared/components/Icon';
import { SelectionItem } from '../../../../../shared/components/SelectionItem/SelectionItem';

interface Props {
  instanceId: number;
  instanceName: string;
  selected: boolean;
  disabled?: boolean;
  onClick?: () => void;
}

export const InstanceSelector = ({
  instanceId,
  instanceName,
  selected,
  onClick,
  disabled = false,
}: Props) => {
  return (
    <SelectionItem
      className="instance-selector"
      data-instance-id={instanceId}
      text={instanceName}
      icon={IconKind.Globe}
      selected={selected}
      disabled={disabled}
      onClick={onClick}
    />
  );
};
