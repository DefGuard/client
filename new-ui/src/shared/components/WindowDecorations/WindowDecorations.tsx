import './style.scss';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { type as getOsType } from '@tauri-apps/plugin-os';
import clsx from 'clsx';
import { useEffect, useState } from 'react';
import { WindowId } from '../../consts';

const osType = getOsType();

const isWindows = osType === 'windows';
const isMac = osType === 'macos';
const osCheck = isWindows || isMac;

const appWindow = getCurrentWindow();

const isFullView = appWindow.label === WindowId.FullView;

const decorationsHeight = 33;

export const WindowDecorations = () => {
  const [isMaximized, setIsMaximized] = useState(false);
  const [isDecorated, setIsDecorated] = useState(true);

  useEffect(() => {
    void appWindow.isDecorated().then(setIsDecorated);
  }, []);

  useEffect(() => {
    document.documentElement.style.setProperty(
      '--window-decorations-height',
      !isDecorated && isFullView && osCheck ? `${decorationsHeight}px` : '0',
    );
  }, [isDecorated]);

  useEffect(() => {
    if (!osCheck || !isFullView || isDecorated) return;

    void appWindow.isMaximized().then(setIsMaximized);

    const unlisten = appWindow.onResized(() => {
      void appWindow.isMaximized().then(setIsMaximized);
    });

    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [isDecorated]);

  if (isDecorated || !isFullView) {
    return null;
  }

  return (
    <div
      id="window-decorations"
      className={clsx({
        widows: isWindows,
        macos: isMac,
      })}
    >
      <div className="window-drag" data-tauri-drag-region></div>
      <div className="window-controls">
        <button
          className="minimize"
          title="Minimize"
          aria-label="Minimize"
          onClick={() => void appWindow.minimize()}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="10"
            height="2"
            viewBox="0 0 10 2"
            fill="none"
          >
            <path d="M0 0.625H10" stroke="white" strokeWidth="1.25" />
          </svg>
        </button>
        <button
          className="maximize"
          title={isMaximized ? 'Restore' : 'Maximize'}
          aria-label={isMaximized ? 'Restore' : 'Maximize'}
          onClick={() =>
            void (isMaximized ? appWindow.unmaximize() : appWindow.maximize())
          }
        >
          {isMaximized && (
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <rect
                x="2.6"
                y="0.6"
                width="8.8"
                height="8.8"
                rx="1.4"
                stroke="white"
                strokeWidth="1.2"
              />
              <rect
                x="0.6"
                y="2.6"
                width="8.8"
                height="8.8"
                rx="1.4"
                stroke="white"
                strokeWidth="1.2"
                fill="var(--bg, transparent)"
              />
            </svg>
          )}
          {!isMaximized && (
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <rect
                x="0.599976"
                y="0.599976"
                width="10"
                height="10"
                rx="2"
                stroke="white"
                strokeWidth="1.2"
              />
            </svg>
          )}
        </button>
        <button
          className="close"
          title="Close"
          aria-label="Close"
          onClick={() => void appWindow.close()}
        >
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
            fill="none"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M0.424255 9.42426L0.848519 9.84852L9.84849 0.84853L9.42423 0.424265L8.99997 6.48361e-07L-8.31383e-06 8.99999L0.424255 9.42426Z"
              fill="white"
            />
            <path
              d="M0.431763 0.424255L0.00749901 0.84852L9.00747 9.84851L9.43174 9.42425L9.856 8.99998L0.856026 -9.10061e-06L0.431763 0.424255Z"
              fill="white"
            />
          </svg>
        </button>
      </div>
    </div>
  );
};
