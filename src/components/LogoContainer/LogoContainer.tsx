import './style.scss';

import SvgDefguardLogoText from '../../shared/components/svg/DefguardLogoText';
import SvgTeoniteLogo from '../../shared/components/svg/TeoniteLogo';
import { Divider } from '../../shared/defguard-ui/components/Layout/Divider/Divider';
import { DividerDirection } from '../../shared/defguard-ui/components/Layout/Divider/types';

export const LogoContainer = () => {
  return (
    <div className="logo-container">
      <SvgDefguardLogoText className="defguard" />
      <Divider direction={DividerDirection.VERTICAL} />
      <SvgTeoniteLogo className="teonite" />
    </div>
  );
};
