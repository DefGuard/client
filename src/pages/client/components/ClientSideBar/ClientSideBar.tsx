import './style.scss';

import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguardLogoCollapsed from '../../../../shared/components/svg/DefguardLogoCollapsed';
import SvgDefguardLogoIcon from '../../../../shared/components/svg/DefguardLogoIcon';
import SvgDefguardLogoText from '../../../../shared/components/svg/DefguardLogoText';
import SvgIconNavConnections from '../../../../shared/components/svg/IconNavConnections';
import { deviceBreakpoints } from '../../../../shared/constants';
import { IconContainer } from '../../../../shared/defguard-ui/components/Layout/IconContainer/IconContainer';
import SvgIconPlus from '../../../../shared/defguard-ui/components/svg/IconPlus';
import { useClientStore } from '../../hooks/useClientStore';
import { useAddInstanceModal } from '../modals/AddInstanceModal/hooks/useAddInstanceModal';
import { ClientBarItem } from './components/ClientBarItem/ClientBarItem';

const ClientSideBarMobile = () => {
  const instances = useClientStore((state) => state.instances);

  return (
    <div id="client-page-side">
      <div className="logo">
        <SvgDefguardLogoCollapsed />
      </div>
      <div className="items">
        <div className="client-bar-item active">
          <SvgIconNavConnections />
        </div>
        {instances.map((instance) => (
          <ClientBarItem instance={instance} key={instance.id} />
        ))}
        <AddInstance />
      </div>
    </div>
  );
};

export const ClientSideBar = () => {
  const { LL } = useI18nContext();
  const instances = useClientStore((state) => state.instances);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  return breakpoint === 'desktop' ? (
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
        {instances.map((instance) => (
          <ClientBarItem instance={instance} key={instance.id} />
        ))}
        <AddInstance />
      </div>
    </div>
  ) : (
    <ClientSideBarMobile />
  );
};

const AddInstance = () => {
  const { LL } = useI18nContext();
  const openAddInstanceModal = useAddInstanceModal((state) => state.open);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  return (
    <div className="client-bar-item clickable" onClick={() => openAddInstanceModal()}>
      <IconContainer>
        <SvgIconPlus />
      </IconContainer>
      {breakpoint === 'desktop' && <p>{LL.pages.client.sideBar.addInstance()}</p>}
    </div>
  );
};
