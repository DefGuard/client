import './style.scss';
import { ThemeVariable } from '../../types';
import { Icon, IconKind, type IconKindValue } from '../Icon';

interface Props {
  icon?: IconKindValue;
  message: string;
}

export const InfoBanner = ({ message, icon = IconKind.InfoFilled }: Props) => {
  return (
    <div className="info-banner">
      <div className="grid">
        <div className="icon-track">
          <Icon icon={icon} size={20} staticColor={ThemeVariable.FgWhite100} />
        </div>
        <div className="content-track">
          <p>{message}</p>
        </div>
      </div>
    </div>
  );
};
