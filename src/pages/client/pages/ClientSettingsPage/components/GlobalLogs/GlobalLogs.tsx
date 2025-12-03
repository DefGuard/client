import './style.scss';

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as clipboard from '@tauri-apps/plugin-clipboard-manager';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef } from 'react';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ActionButton } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { clientApi } from '../../../../clientAPI/clientApi';
import type {
  GlobalLogLevel,
  LogItem,
  LogLevel,
  LogSource,
} from '../../../../clientAPI/types';
import { GlobalLogsSelect } from './GlobalLogsSelect';
import { GlobalLogsSourceSelect } from './GlobalLogsSourceSelect';

export const GlobalLogs = () => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const globalLogLevelRef = useRef<GlobalLogLevel>('INFO');
  const logSourceRef = useRef<LogSource>('All');
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details.logs;
  const { startGlobalLogWatcher, stopGlobalLogWatcher } = clientApi;

  const handleLogsDownload = async () => {
    try {
      const path = await save({
        filters: [
          {
            name: 'Logs',
            extensions: ['txt', 'log'],
          },
        ],
      });

      if (path) {
        const logs = getAllLogs();
        await writeTextFile(path, logs);
      } else {
        throw new Error('No path selected');
      }
    } catch (e) {
      error(`Failed to save logs: ${e}`);
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
                filterLogByLevel(globalLogLevelRef.current, item.level) &&
                filterLogBySource(logSourceRef.current, item.source)
              ) {
                const utcTimestamp = item.timestamp.endsWith('Z')
                  ? item.timestamp
                  : `${item.timestamp}Z`;
                const dateTime = new Date(utcTimestamp).toLocaleString();
                const messageString = `[${dateTime}][${item.level}][${item.source}] ${item.fields.message}`;
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
  }, [startGlobalLogWatcher, stopGlobalLogWatcher]);

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
    <Card shaded={false} id="global-logs" bordered>
      <div className="top">
        <h3>{localLL.title()}</h3>
        <div id="selects">
          <GlobalLogsSelect
            initSelected={'INFO'}
            onChange={(level) => {
              globalLogLevelRef.current = level;
              clearLogs();
              stopGlobalLogWatcher();
              startGlobalLogWatcher();
            }}
          />
          <div className="select-with-helper">
            <GlobalLogsSourceSelect
              initSelected={'All'}
              onChange={(source) => {
                logSourceRef.current = source;
                clearLogs();
                stopGlobalLogWatcher();
                startGlobalLogWatcher();
              }}
            />
            <Helper>
              <p>
                {LL.pages.client.pages.settingsPage.tabs.global.globalLogs.logSourceHelper()}
              </p>
            </Helper>
          </div>
        </div>
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
    case 'INFO':
      return ['info', 'error', 'warn'].includes(log_level);
    case 'DEBUG':
      return ['error', 'info', 'debug', 'warn'].includes(log_level);
    default:
      return true;
  }
};

const filterLogBySource = (target: LogSource, log: LogSource): boolean => {
  return target === 'All' || target === log;
};
