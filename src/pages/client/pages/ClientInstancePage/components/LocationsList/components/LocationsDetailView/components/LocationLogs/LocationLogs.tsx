import './style.scss';

import { clipboard } from '@tauri-apps/api';
import { save } from '@tauri-apps/api/dialog';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { writeTextFile } from '@tauri-apps/api/fs';
import { isUndefined } from 'lodash-es';
import { useEffect, useRef } from 'react';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LogItem, LogLevel } from '../../../../../../../../clientAPI/types';
import { useClientStore } from '../../../../../../../../hooks/useClientStore';
import { DefguardLocation, WireguardInstanceType } from '../../../../../../../../types';
import { LocationLogsSelect } from './LocationLogsSelect';

type Props = {
  locationId: DefguardLocation['id'];
  locationType: WireguardInstanceType;
};

export const LocationLogs = ({ locationId }: Props) => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const appLogLevel = useClientStore((state) => state.settings.log_level);
  const locationLogLevelRef = useRef<LogLevel>(appLogLevel);
  const logsRef = useRef('');
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details.logs;

  const handleLogsDownload = async () => {
    const path = await save({});
    if (path) {
      await writeTextFile(path, logsRef.current);
    }
  };

  // Listen to new logs
  useEffect(() => {
    let eventUnlisten: UnlistenFn;
    const startLogListen = async () => {
      eventUnlisten = await listen<LogItem[]>(
        `log-update-location-${locationId}-${location}`,
        ({ payload: logItems }) => {
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
        <LocationLogsSelect
          initSelected={appLogLevel}
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
