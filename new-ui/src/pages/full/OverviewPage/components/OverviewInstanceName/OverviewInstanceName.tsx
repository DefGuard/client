import './style.scss';
import { TruncatedText } from '../../../../../shared/components/TruncatedText/TruncatedText';

type Props = {
  name: string;
};

export const OverviewInstanceName = ({ name }: Props) => {
  return (
    <TruncatedText
      className="overview-instance-name"
      text={name}
      tooltipPlacement="bottom-start"
      delayTooltip
    />
  );
};
