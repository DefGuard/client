import './style.scss';

import SvgDefguardLogoIcon from '../../../../shared/components/svg/DefguardLogoIcon';
import SvgDefguardLogoText from '../../../../shared/components/svg/DefguardLogoText';

export const ClientSideBar = () => {
  return (
    <div id="client-page-side">
      <div className="logo">
        <SvgDefguardLogoIcon />
        <SvgDefguardLogoText />
      </div>
      <div className="items"></div>
    </div>
  );
};
