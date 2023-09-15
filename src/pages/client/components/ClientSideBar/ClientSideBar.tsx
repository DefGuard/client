import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguardLogoIcon from '../../../../shared/components/svg/DefguardLogoIcon';
import SvgDefguardLogoText from '../../../../shared/components/svg/DefguardLogoText';
import SvgIconNavConnections from '../../../../shared/components/svg/IconNavConnections';
import { IconContainer } from '../../../../shared/defguard-ui/components/Layout/IconContainer/IconContainer';
import SvgIconPlus from '../../../../shared/defguard-ui/components/svg/IconPlus';
import { useAddInstanceModal } from '../modals/AddInstanceModal/hooks/useAddInstanceModal';

export const ClientSideBar = () => {
  const { LL } = useI18nContext();
  return (
    <div id="client-page-side">
      <div className="logo">
        <SvgDefguardLogoIcon />
        <SvgDefguardLogoText />
      </div>
      <div className="items">
        <div className="client-bar-item active">
          <SvgIconNavConnections />
          <p>{LL.pages.client.sideBar.instances()}</p>
        </div>
        <AddInstance />
      </div>
    </div>
  );
};

const AddInstance = () => {
  const { LL } = useI18nContext();
  const openAddInstanceModal = useAddInstanceModal((state) => state.open);
  return (
    <div className="client-bar-item clickable" onClick={() => openAddInstanceModal()}>
      <IconContainer>
        <SvgIconPlus />
      </IconContainer>
      <p>{LL.pages.client.sideBar.addInstance()}</p>
    </div>
  );
};
