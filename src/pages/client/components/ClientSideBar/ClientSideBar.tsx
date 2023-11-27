import './style.scss';

import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguadNavLogoCollapsed from '../../../../shared/components/svg/DefguardLogoCollapsed';
import SvgDefguardLogoIcon from '../../../../shared/components/svg/DefguardLogoIcon';
import SvgDefguardLogoText from '../../../../shared/components/svg/DefguardLogoText';
import SvgIconNavConnections from '../../../../shared/components/svg/IconNavConnections';
import { IconContainer } from '../../../../shared/defguard-ui/components/Layout/IconContainer/IconContainer';
import SvgIconPlus from '../../../../shared/defguard-ui/components/svg/IconPlus';
import { routes } from '../../../../shared/routes';
import { useClientStore } from '../../hooks/useClientStore';
import { ClientBarItem } from './components/ClientBarItem/ClientBarItem';

export const ClientSideBar = () => {
  const { LL } = useI18nContext();
  const instances = useClientStore((state) => state.instances);

  return (
    <div id="client-page-side">
      <div className="logo-desktop">
        <SvgDefguardLogoIcon />
        <SvgDefguardLogoText />
      </div>
      <div className="logo-mobile">
        <SvgDefguadNavLogoCollapsed />
      </div>
      <div className="items">
        <div className="client-bar-item active" id="instances-nav-label">
          <SvgIconNavConnections />
          <p>{LL.pages.client.sideBar.instances()}</p>
        </div>
        {instances.map((instance) => (
          <ClientBarItem instance={instance} key={instance.id} />
        ))}
        <AddInstance />
      </div>
    </div>
  );
};

const AddInstance = () => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  return (
    <div
      id="add-instance"
      className="client-bar-item clickable"
      onClick={() => {
        navigate(routes.client.addInstance, { replace: true });
      }}
    >
      <IconContainer>
        <SvgIconPlus />
      </IconContainer>
      <p>{LL.pages.client.sideBar.addInstance()}</p>
    </div>
  );
};
