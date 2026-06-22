import clsx from 'clsx';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { getVersion } from '@tauri-apps/api/app';
import { useId } from 'react';
import { isPresent } from '../../utils/isPresent';
import { ConnectionWatcher } from './components/ConnectionWatcher/ConnectionsWatcher';

interface Props {
  variant: 'compact' | 'desktop';
}

export const WindowHeader = ({ variant }: Props) => {
  const { data: appVersion } = useQuery({
    queryFn: getVersion,
    queryKey: ['app-version'],
  });

  const version = () => {
    if (appVersion) {
      return `Version ${appVersion}`;
    }
  };

  return (
    <div id="window-header" className={clsx(`variant-${variant}`)}>
      <LogoIcon size={variant === 'desktop' ? 33 : 48} />
      <div className="info">
        <p className="label">Defguard VPN Client</p>
        {variant === 'compact' && <ConnectionWatcher />}
        {variant === 'desktop' && isPresent(appVersion) && (
          <p className="version">{version()}</p>
        )}
      </div>
      {variant === 'desktop' && (
        <div className="right">
          <ConnectionWatcher />
        </div>
      )}
    </div>
  );
};

const LogoIcon = ({ size = 48 }: { size?: number }) => {
  const id = useId();
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 48 48"
      fill="none"
    >
      <rect width="48" height="48" rx="12" fill={`url(#${id})`} />
      <path
        d="M29.3374 7V13.9321L23.5061 10.466L16 14.9251V31.7619L23.5 36.2209L29.3313 32.7549V36.3271L26.5097 38.007L28.1784 39L31 37.3201V22.8439L23.5 18.3849L17.6687 21.8509V15.9118L23.5 12.4457L29.3313 15.9118V17.8852L31 18.8782V7.99297L29.3313 7H29.3374ZM17.6687 30.7689V24.8298L23.5 28.2959L29.3313 24.8298V30.7689L23.5 34.235L17.6687 30.7689ZM28.5 23.3435L23.5 26.3162L18.5 23.3435L23.5 20.3708L28.5 23.3435Z"
        fill="#3961DB"
      />
      <defs>
        <linearGradient
          id={id}
          x1="24"
          y1="0"
          x2="24"
          y2="48"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="white" />
          <stop offset="1" stopColor="#D3DDFB" />
        </linearGradient>
      </defs>
    </svg>
  );
};
