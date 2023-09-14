import './style.scss';

import classNames from 'classnames';

import SvgIconCheckmarkSmall from '../../../../../../shared/components/svg/IconCheckmarkSmall';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonSize } from '../../../../../../shared/defguard-ui/components/Layout/Button/types';

export const LocationItem = () => {
  const cn = classNames('location-item');

  return (
    <div className={cn}>
      <div className="top">
        <p className="name">Location name placeholder</p>
        <Button size={ButtonSize.SMALL} icon={<SvgIconCheckmarkSmall />} text="Connect" />
      </div>
      <p className="no-data">
        This device was never connected to this location, connect to view statistics and
        information about connection
      </p>
    </div>
  );
};
