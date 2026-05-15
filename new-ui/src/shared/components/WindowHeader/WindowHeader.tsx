import clsx from 'clsx';
import './style.scss';

interface Props {
  variant: 'compact' | 'desktop';
}

export const WindowHeader = ({ variant }: Props) => {
  return (
    <div id="window-header" className={clsx(`variant-${variant}`)}>
      <LogoIcon />
      <div className="info">
        <p>Defguard VPN Client</p>
      </div>
    </div>
  );
};

const LogoIcon = () => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="48"
      height="48"
      viewBox="0 0 48 48"
      fill="none"
    >
      <rect width="48" height="48" rx="12" fill="url(#paint0_linear_287_5972)" />
      <path
        d="M29.3374 7V13.9321L23.5061 10.466L16 14.9251V31.7619L23.5 36.2209L29.3313 32.7549V36.3271L26.5097 38.007L28.1784 39L31 37.3201V22.8439L23.5 18.3849L17.6687 21.8509V15.9118L23.5 12.4457L29.3313 15.9118V17.8852L31 18.8782V7.99297L29.3313 7H29.3374ZM17.6687 30.7689V24.8298L23.5 28.2959L29.3313 24.8298V30.7689L23.5 34.235L17.6687 30.7689ZM28.5 23.3435L23.5 26.3162L18.5 23.3435L23.5 20.3708L28.5 23.3435Z"
        fill="#3961DB"
      />
      <defs>
        <linearGradient
          id="paint0_linear_287_5972"
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
