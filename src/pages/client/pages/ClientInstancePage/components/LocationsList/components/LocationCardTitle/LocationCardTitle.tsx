import './style.scss';

import classNames from 'classnames';

import { Badge } from '../../../../../../../../shared/defguard-ui/components/Layout/Badge/Badge';
import { BadgeStyleVariant } from '../../../../../../../../shared/defguard-ui/components/Layout/Badge/types';
import SvgIconConnection from '../../../../../../../../shared/defguard-ui/components/svg/IconConnection';
import { CommonWireguardFields } from '../../../../../../types';

type Props = {
  location?: CommonWireguardFields;
};

export const LocationCardTitle = ({ location }: Props) => {
  const cn = classNames('location-card-title', {
    active: location?.active,
  });
  return (
    <div className={cn}>
      <SvgIconConnection />
      <span className="location-name">{location?.name}</span>
      <Badge text={location?.address || ''} styleVariant={BadgeStyleVariant.STANDARD} />
    </div>
  );
};
