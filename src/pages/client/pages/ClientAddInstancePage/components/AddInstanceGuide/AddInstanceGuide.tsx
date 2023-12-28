import './style.scss';

import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import SvgVpnLocation from '../../../../../../shared/components/svg/VpnLocation';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';

export const AddInstanceGuide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addInstancePage.guide;
  return (
    <section id="add-instance-guide">
      <div id="instance-guide">
        <h2>{localLL.title()}</h2>
        <p>{localLL.subTitle()}</p>
      </div>
      <SvgVpnLocation />
      <Card id="token-guide">
        <h2>{localLL.card.title()}</h2>
        {parse(localLL.card.content())}
      </Card>
    </section>
  );
};
