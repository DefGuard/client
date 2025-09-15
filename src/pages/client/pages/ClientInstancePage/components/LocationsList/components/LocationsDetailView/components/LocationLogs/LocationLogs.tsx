import './style.scss';

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as clipboard from '@tauri-apps/plugin-clipboard-manager';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useRef } from 'react';
import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import type { LogItem, LogLevel } from '../../../../../../../../clientAPI/types';
import { useClientStore } from '../../../../../../../../hooks/useClientStore';
import type {
  DefguardLocation,
  ClientConnectionType,
} from '../../../../../../../../types';
import { LocationLogsSelect } from './LocationLogsSelect';

type Props = {
  locationId: DefguardLocation['id'];
  connectionType: ClientConnectionType;
};

export const LocationLogs = ({ locationId, connectionType }: Props) => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const appLogLevel = useClientStore((state) => state.appConfig.log_level);
  const locationLogLevelRef = useRef<LogLevel>(appLogLevel);
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details.logs;

  const handleLogsDownload = async () => {
    const path = await save({});
    if (path) {
      const logs = getAllLogs();
      await writeTextFile(path, logs);
    }
  };

  const clearLogs = useCallback(() => {
    if (logsContainerElement.current) {
      logsContainerElement.current.innerHTML = '';
    }
  }, []);

  // Clear logs when the component is unmounted or locationId changes
  useEffect(() => {
    return () => clearLogs();
  }, [clearLogs]);

  // Listen to new logs
  useEffect(() => {
    let eventUnlisten: UnlistenFn;
    const startLogListen = async () => {
      eventUnlisten = await listen<LogItem[]>(
        `log-update-${connectionType}-${locationId}`,
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
  }, [locationId, connectionType]);

  const getAllLogs = () => {
    let logs = '';

    if (logsContainerElement) {
      logsContainerElement.current?.childNodes.forEach((item) => {
        logs += `${item.textContent}\n`;
      });
    }

    return logs;
  };

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
            const logs = getAllLogs();
            if (logs) {
              clipboard.writeText(logs);
            }
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
  const log_level = log.toLocaleLowerCase();
  switch (target) {
    case 'ERROR':
      return log_level === 'error';
    case 'WARN':
      return ['error', 'warn'].includes(log_level);
    case 'INFO':
      return ['info', 'error', 'warn'].includes(log_level);
    case 'DEBUG':
      return ['error', 'info', 'debug', 'warn'].includes(log_level);
    case 'TRACE':
      return true;
  }
};
