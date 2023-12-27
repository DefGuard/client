import './style.scss';

import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import SvgVpnLocation from '../../../../../shared/components/svg/VpnLocation';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';

export const AddTunnelGuide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addTunnelPage.guide;
  return (
    <section id="add-tunnel-guide">
      <div id="tunnel-guide">
        <h2>{localLL.title()}</h2>
        {parse(localLL.subTitle())}
      </div>
      <SvgVpnLocation />
      <Card id="setup-guide">
        <h2>{localLL.card.title()}:</h2>
        {parse(localLL.card.content())}
      </Card>
    </section>
  );
};
