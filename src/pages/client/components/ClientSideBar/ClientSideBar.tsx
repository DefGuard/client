import './style.scss';

import classNames from 'classnames';
import { useMatch, useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguadNavLogoCollapsed from '../../../../shared/components/svg/DefguardLogoCollapsed';
import SvgDefguardLogoIcon from '../../../../shared/components/svg/DefguardLogoIcon';
import SvgDefguardLogoText from '../../../../shared/components/svg/DefguardLogoText';
import SvgIconNavConnections from '../../../../shared/components/svg/IconNavConnections';
import { IconContainer } from '../../../../shared/defguard-ui/components/Layout/IconContainer/IconContainer';
import SvgIconPlus from '../../../../shared/defguard-ui/components/svg/IconPlus';
import SvgIconSettings from '../../../../shared/defguard-ui/components/svg/IconSettings';
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
        <SettingsNav />
      </div>
    </div>
  );
};

const SettingsNav = () => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const pathActive = useMatch(routes.client.settings);
  return (
    <div
      id="settings-nav-item"
      className={classNames('client-bar-item clickable', {
        active: pathActive !== null,
      })}
      onClick={() => {
        navigate(routes.client.settings, { replace: true });
      }}
    >
      <SvgIconSettings />
      <p>{LL.pages.client.sideBar.settings()}</p>
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
