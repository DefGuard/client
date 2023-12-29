import './style.scss';

import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { isUndefined } from 'lodash-es';
import { useEffect, useRef } from 'react';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LogItem } from '../../../../../../../../clientAPI/types';
import { DefguardLocation } from '../../../../../../../../types';

type Props = {
  locationId: DefguardLocation['id'];
};

export const LocationLogs = ({ locationId }: Props) => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details.logs;

  // Listen to new logs
  useEffect(() => {
    let eventUnlisten: UnlistenFn;
    const startLogListen = async () => {
      eventUnlisten = await listen<LogItem[]>(
        `log-update-location-${locationId}`,
        ({ payload: logItems }) => {
          if (logsContainerElement.current) {
            logItems.forEach((item) => {
              if (logsContainerElement.current) {
                const messageString = `${item.timestamp} ${item.level} ${item.fields.message}`;
                const element = createLogLineElement(messageString);
                const scrollAfterAppend =
                  logsContainerElement.current.scrollHeight -
                    logsContainerElement.current.scrollTop ===
                  logsContainerElement.current.clientHeight;
                logsContainerElement.current.appendChild(element);
                // auto scroll to bottom if user didn't scroll up
                if (scrollAfterAppend) {
                  logsContainerElement.current.scrollTo({
                    top: logsContainerElement.current.scrollHeight,
                  });
                }
              }
            });
          }
        },
      );
    };
    if (!isUndefined(locationId)) {
      startLogListen();
    }
    //unsubscribe on dismount
    return () => {
      eventUnlisten?.();
    };
    //eslint-disable-next-line
  }, [locationId]);

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
