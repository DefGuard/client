import './style.scss';

import { clipboard } from '@tauri-apps/api';
import { save } from '@tauri-apps/api/dialog';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { writeTextFile } from '@tauri-apps/api/fs';
import { useEffect, useRef } from 'react';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { clientApi } from '../../../../../../../../clientAPI/clientApi';
import { LogItem, LogLevel } from '../../../../../../../../clientAPI/types';
import { DefguardLocation } from '../../../../../../../../types';
import { LocationLogsSelect } from './LocationLogsSelect';

const { getLocationInterfaceLogs, stopLocationInterfaceLogs } = clientApi;

type Props = {
  locationId: DefguardLocation['id'];
};

export const LocationLogs = ({ locationId }: Props) => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const locationLogLevelRef = useRef<LogLevel>('info');
  const logsRef = useRef('');
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details.logs;

  const handleLogsDownload = async () => {
    const path = await save({});
    if (path) {
      await writeTextFile(path, logsRef.current);
    }
  };

  // mount logger and stream log elements into log-container
  useEffect(() => {
    let eventUnlisten: UnlistenFn;
    const startLogging = async () => {
      const eventTopic = await getLocationInterfaceLogs({ locationId });
      // assign unlisten
      eventUnlisten = await listen<LogItem[]>(eventTopic, ({ payload: logItems }) => {
        if (logsContainerElement.current) {
          logItems.forEach((item) => {
            if (
              logsContainerElement.current &&
              filterLogByLevel(locationLogLevelRef.current, item.level)
            ) {
              const messageString = `${item.timestamp} ${item.level} ${item.fields.message}`;
              const element = createLogLineElement(messageString);
              const scrollAfterAppend =
                logsContainerElement.current.scrollHeight -
                  logsContainerElement.current.scrollTop ===
                logsContainerElement.current.clientHeight;
              logsContainerElement.current.appendChild(element);
              logsRef.current += messageString + '\n';
              // auto scroll to bottom if user didn't scroll up
              if (scrollAfterAppend) {
                logsContainerElement.current.scrollTo({
                  top: logsContainerElement.current.scrollHeight,
                });
              }
            }
          });
        }
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
        <LocationLogsSelect
          initSelected={'info'}
          onChange={(level) => {
            locationLogLevelRef.current = level;
          }}
        />
        <ActionButton
          variant={ActionButtonVariant.COPY}
          onClick={() => {
            clipboard.writeText(logsRef.current);
          }}
        />
        <ActionButton
          variant={ActionButtonVariant.DOWNLOAD}
          onClick={handleLogsDownload}
        />
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

// return true if log should be visible
const filterLogByLevel = (target: LogLevel, log: LogLevel): boolean => {
  switch (target) {
    case 'error':
      return log === 'error';
    case 'info':
      return ['info', 'error'].includes(log);
    case 'debug':
      return ['error', 'info', 'debug'].includes(log);
    case 'trace':
      return true;
  }
};
