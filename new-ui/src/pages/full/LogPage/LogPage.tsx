import './style.scss';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as clipboard from '@tauri-apps/plugin-clipboard-manager';
import { save } from '@tauri-apps/plugin-dialog';
import { writeTextFile } from '@tauri-apps/plugin-fs';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Subject } from 'rxjs';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { Select } from '../../../shared/components/Select/Select';
import type { SelectOption } from '../../../shared/components/Select/types';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { TooltipButton } from '../../../shared/components/TooltipButton/TooltipButton';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { api } from '../../../shared/rust-api/api';
import {
  type LogItem,
  type LogLevel,
  LogSource,
  TauriEvent,
} from '../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../shared/types';

type GlobalLogLevel = Extract<LogLevel, 'ERROR' | 'INFO' | 'DEBUG'>;

const logLevelOptions: SelectOption<GlobalLogLevel>[] = [
  { key: 'error', label: 'Error', value: 'ERROR' },
  { key: 'info', label: 'Info', value: 'INFO' },
  { key: 'debug', label: 'Debug', value: 'DEBUG' },
];

const logSourceOptions: SelectOption<LogSource>[] = [
  { key: 'all', label: 'All', value: LogSource.All },
  { key: 'client', label: 'Client', value: LogSource.Client },
  { key: 'vpn', label: 'VPN', value: LogSource.Vpn },
];

const filterLogByLevel = (target: GlobalLogLevel, log: LogLevel): boolean => {
  switch (target) {
    case 'ERROR':
      return log === 'ERROR';
    case 'INFO':
      return ['INFO', 'ERROR', 'WARN'].includes(log);
    case 'DEBUG':
      return ['ERROR', 'INFO', 'DEBUG', 'WARN'].includes(log);
    default:
      return true;
  }
};

const filterLogBySource = (target: LogSource, log: LogSource): boolean =>
  target === LogSource.All || target === log;

const createLogLineElement = (content: string): HTMLParagraphElement => {
  const element = document.createElement('p');
  element.classList.add('log-line');
  element.textContent = content;
  return element;
};

export const LogPage = () => {
  const logsContainerElement = useRef<HTMLDivElement | null>(null);
  const logLevelRef = useRef<GlobalLogLevel>('INFO');
  const logSourceRef = useRef<LogSource>(LogSource.All);
  const [level, setLevel] = useState(logLevelOptions[1]);
  const [source, setSource] = useState(logSourceOptions[0]);

  const clearLogs = useCallback(() => {
    if (logsContainerElement.current) {
      logsContainerElement.current.innerHTML = '';
    }
  }, []);

  const restartLogWatcher = useCallback(() => {
    clearLogs();
    api.stopGlobalLogWatcher();
    api.startGlobalLogWatcher();
  }, [clearLogs]);

  const getAllLogs = useCallback(() => {
    let logs = '';
    logsContainerElement.current?.childNodes.forEach((item) => {
      logs += `${item.textContent}\n`;
    });
    return logs;
  }, []);

  const clipboardSub = useRef(new Subject<void>());
  const downloadSub = useRef(new Subject<void>());

  const handleLogsCopy = useCallback(() => {
    const logs = getAllLogs();
    if (logs) {
      clipboard.writeText(logs).then(() => {
        clipboardSub.current.next();
      });
    }
  }, [getAllLogs]);

  const handleLogsDownload = useCallback(async () => {
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
        await writeTextFile(path, getAllLogs());
        downloadSub.current.next();
      }
    } catch (e) {
      error(`Failed to save logs to file: ${String(e)}`);
    }
  }, [getAllLogs]);

  useEffect(() => {
    let eventUnlisten: UnlistenFn;
    const startLogListen = async () => {
      eventUnlisten = await listen<LogItem[]>(
        TauriEvent.GlobalLogUpdate,
        ({ payload: logItems }) => {
          const container = logsContainerElement.current;
          if (!container) return;
          for (const item of logItems) {
            if (
              filterLogByLevel(logLevelRef.current, item.level) &&
              filterLogBySource(logSourceRef.current, item.source)
            ) {
              const utcTimestamp = item.timestamp.endsWith('Z')
                ? item.timestamp
                : `${item.timestamp}Z`;
              const dateTime = new Date(utcTimestamp).toLocaleString();
              const element = createLogLineElement(
                `[${dateTime}][${item.level}][${item.source}] ${item.fields.message}`,
              );
              // stick to bottom unless the user scrolled up to read
              const scrollAfterAppend =
                container.scrollHeight - container.scrollTop - container.clientHeight <
                10;
              container.appendChild(element);
              // auto scroll to bottom if user didn't scroll up
              if (scrollAfterAppend) {
                container.scrollTo({ top: container.scrollHeight });
              }
            }
          }
        },
      );
    };
    startLogListen();
    api.startGlobalLogWatcher();

    return () => {
      api.stopGlobalLogWatcher();
      eventUnlisten?.();
      clearLogs();
    };
  }, [clearLogs]);

  return (
    <FullPage id="log-page-view" hideScrollContainer>
      <FullPageTitle title="Log" spacing={ThemeSpacing.Sm} />
      <p className="page-description">
        The source of the logs. Logs can come from the Defguard client or the VPN
        service/extension that manages VPN connections at the network level.
      </p>
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="controls">
        <Select
          options={logLevelOptions}
          value={level}
          onChange={(option) => {
            setLevel(option);
            logLevelRef.current = option.value;
            restartLogWatcher();
          }}
        />
        <Select
          options={logSourceOptions}
          value={source}
          onChange={(option) => {
            setSource(option);
            logSourceRef.current = option.value;
            restartLogWatcher();
          }}
        />
        <div className="spacer" />
        <TooltipButton
          tooltipTrigger={downloadSub.current}
          tooltipText="Logs downloaded"
          buttonProps={{
            variant: ButtonVariant.Outlined,
            text: 'Download',
            iconLeft: 'download',
            onClick: handleLogsDownload,
          }}
        />
        <TooltipButton
          tooltipTrigger={clipboardSub.current}
          tooltipText="Logs copied to clipboard"
          buttonProps={{
            variant: ButtonVariant.Outlined,
            text: 'Copy to Clipboard',
            iconLeft: 'copy',
            onClick: handleLogsCopy,
          }}
        />
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <div ref={logsContainerElement} className="log-container" />
    </FullPage>
  );
};
