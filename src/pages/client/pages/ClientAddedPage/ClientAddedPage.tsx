import './style.scss';

import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgVpnLocation from '../../../../shared/components/svg/VpnLocation';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { routes } from '../../../../shared/routes';
import { WireguardInstanceType } from '../../types';

type Props = {
  pageType: WireguardInstanceType;
};

export const ClientAddedPage = ({ pageType }: Props) => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const [localLL, navigateRoute] =
    pageType === WireguardInstanceType.TUNNEL
      ? [LL.pages.client.pages.createdPage.tunnel, routes.client.addTunnel]
      : [LL.pages.client.pages.createdPage.instance, routes.client.addInstance];

  return (
    <section className="client-page" id="created-page">
      <div className="content">
        <Card id="created">
          <div className="card-content">
            <h2>{localLL.title()}</h2>
            <SvgVpnLocation />
            <p>{localLL.content()}</p>
            <Button
              className="submit"
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.PRIMARY}
              onClick={() => navigate(navigateRoute, { replace: true })}
              text={localLL.controls.submit()}
            />
          </div>
        </Card>
      </div>
    </section>
  );
};
