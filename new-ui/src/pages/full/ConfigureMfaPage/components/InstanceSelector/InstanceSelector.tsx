import clsx from 'clsx';
import './style.scss';
import { Icon, IconKind } from '../../../../../shared/components/Icon';
import { RadioIndicator } from '../../../../../shared/components/RadioIndicator/RadioIndicator';
import { ThemeVariable } from '../../../../../shared/types';

interface Props {
  instanceId: number;
  instanceName: string;
  selected: boolean;
  onClick: () => void;
}

export const InstanceSelector = ({
  instanceId,
  instanceName,
  selected,
  onClick,
}: Props) => {
  return (
    <div
      className={clsx('instance-selector', { selected })}
      data-instance-id={instanceId}
      onClick={onClick}
    >
      <div className="track">
        <div className="icon-col side-col">
          <Icon icon={IconKind.Globe} size={20} staticColor={ThemeVariable.FgWhite70} />
        </div>
        <div className="content-col">
          <p className="name">{instanceName}</p>
        </div>
        <div className="selection side-col">
          <RadioIndicator active={selected} />
        </div>
      </div>
    </div>
  );
};
