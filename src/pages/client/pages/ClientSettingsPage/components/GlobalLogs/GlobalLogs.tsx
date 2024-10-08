import './style.scss';

import { clipboard } from '@tauri-apps/api';
import { save } from '@tauri-apps/api/dialog';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { writeTextFile } from '@tauri-apps/api/fs';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useRef } from 'react';

import { LocationLogsSelect } from '../../../ClientInstancePage/components/LocationsList/components/LocationsDetailView/components/LocationLogs/LocationLogsSelect';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { useClientStore } from '../../../../hooks/useClientStore';
import { LogItem, LogLevel } from '../../../../clientAPI/types';
import { ActionButtonVariant } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { ActionButton } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { clientApi } from '../../../../clientAPI/clientApi';
import { GlobalLogsSelect } from './GlobalLogsSelect';

export const GlobalLogs = () => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const appLogLevel = useClientStore((state) => state.settings.log_level);
  const locationLogLevelRef = useRef<LogLevel>(appLogLevel);
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details.logs;
  const { startGlobalLogWatcher, stopGlobalLogWatcher } = clientApi;

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
        `log-update-global`,
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
    startLogListen();
    startGlobalLogWatcher();

    //unsubscribe on dismount
    return () => {
      stopGlobalLogWatcher();
      eventUnlisten?.();
    };
    //eslint-disable-next-line
  }, []);

  const getAllLogs = () => {
    let logs = '';

    if (logsContainerElement) {
      logsContainerElement.current?.childNodes.forEach((item) => {
        logs += item.textContent + '\n';
      });
    }

    return logs;
  };

  return (
    <Card shaded={false} id="global-logs" bordered>
      <div className="top">
        <h3>{localLL.title()}</h3>
        <GlobalLogsSelect
          initSelected={appLogLevel}
          onChange={(level) => {
            locationLogLevelRef.current = level;
            clearLogs();
            stopGlobalLogWatcher();
            startGlobalLogWatcher();
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
    case 'error':
      return log_level === 'error';
    case 'info':
      return ['info', 'error'].includes(log_level);
    case 'debug':
      return ['error', 'info', 'debug'].includes(log_level);
  }
};
