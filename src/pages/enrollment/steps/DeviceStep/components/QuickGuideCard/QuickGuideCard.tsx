import './style.scss';

import { ReactMarkdown } from 'react-markdown/lib/react-markdown';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';

export const QuickGuideCard = () => {
  const { LL } = useI18nContext();

  const cardLL = LL.pages.enrollment.steps.deviceSetup.cards.guide;

  return (
    <Card id="device-setup-guide">
      <h3>{cardLL.title()}</h3>
      <MessageBox message={cardLL.messageBox()} />
      <div className="steps">
        <label>{cardLL.step({ step: 1 })}</label>
        <p>{cardLL.steps.wireguard.content()}</p>
        <Button
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={cardLL.steps.wireguard.button()}
          onClick={() => {
            window.open('https://www.wireguard.com/install/', '_blank');
          }}
        />
        <label>{cardLL.step({ step: 2 })}</label>
        <p>{cardLL.steps.downloadConfig()}</p>
        <label>{cardLL.step({ step: 3 })}</label>
        <p>{cardLL.steps.addTunnel()}</p>
        <label>{cardLL.step({ step: 4 })}</label>
        <p>{cardLL.steps.activate()}</p>
      </div>
      <div className="finish">
        <ReactMarkdown>{cardLL.steps.finish()}</ReactMarkdown>
      </div>
    </Card>
  );
};
