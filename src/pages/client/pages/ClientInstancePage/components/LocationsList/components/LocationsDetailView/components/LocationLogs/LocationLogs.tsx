import './style.scss';

import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useEffect, useRef } from 'react';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { clientApi } from '../../../../../../../../clientAPI/clientApi';
import { LogItem } from '../../../../../../../../clientAPI/types';
import { DefguardLocation } from '../../../../../../../../types';

const { getLocationInterfaceLogs, stopLocationInterfaceLogs } = clientApi;

type Props = {
  locationId: DefguardLocation['id'];
};

export const LocationLogs = ({ locationId }: Props) => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details.logs;

  // mount logger and stream log elements into log-container
  useEffect(() => {
    let eventUnlisten: UnlistenFn;
    const startLogging = async () => {
      const eventTopic = await getLocationInterfaceLogs({ locationId });
      // assign unlisten
      eventUnlisten = await listen<LogItem[]>(eventTopic, ({ payload: logItems }) => {
        logItems.forEach((item) => {
          const messageString = `${item.timestamp} ${item.level} ${item.fields.message}`;
          const element = createLogLineElement(messageString);
          logsContainerElement.current?.appendChild(element);
        });
      });
    };
    startLogging();
    //unsubscribe on dismount
    return () => {
      eventUnlisten?.();
      stopLocationInterfaceLogs({ locationId });
    };
    //eslint-disable-next-line
  }, []);

  return (
    <Card shaded={false} id="location-logs" bordered>
      <div className="top">
        <h3>{localLL.title()}</h3>
        <ActionButton variant={ActionButtonVariant.COPY} />
        <ActionButton variant={ActionButtonVariant.DOWNLOAD} />
      </div>
      <div ref={logsContainerElement} className="logs-container"></div>
    </Card>
  );
};

const createLogLineElement = (content: string): HTMLParagraphElement => {
  const element = document.createElement('p');
  element.classList.add('log-line');
  element.textContent = content;
  return element;
};
