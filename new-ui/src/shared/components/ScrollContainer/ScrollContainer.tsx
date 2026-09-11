import './style.scss';
import { platform } from '@tauri-apps/plugin-os';
import clsx from 'clsx';
import type { PropsWithChildren } from 'react';

const isWindows = platform() === 'windows';

export const ScrollContainer = ({ children }: PropsWithChildren) => {
  return (
    <div
      className={clsx('scroll-container', {
        windows: isWindows,
      })}
    >
      {children}
    </div>
  );
};
