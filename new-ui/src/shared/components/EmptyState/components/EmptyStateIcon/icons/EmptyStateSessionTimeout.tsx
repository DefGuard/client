import { useId } from 'react';

export const EmptyStateSessionTimeout = () => {
  const id = useId();

  return (
    <svg
      width="48"
      height="48"
      viewBox="0 0 48 48"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect
        x="0.5"
        y="0.5"
        width="47"
        height="47"
        rx="23.5"
        stroke="white"
        strokeOpacity="0.4"
        strokeDasharray="2 2"
      />
      <g clipPath={`url(#${id})`}>
        <path
          d="M25.1409 34.9716C30.586 34.9716 35 30.5233 35 25.0361C35 19.5489 30.586 15.1006 25.1409 15.1006C19.6958 15.1006 15.2817 19.5489 15.2817 25.0361C15.2817 30.5233 19.6958 34.9716 25.1409 34.9716Z"
          fill="white"
          fillOpacity="0.1"
        />
        <path
          d="M23.9153 33.9355C29.3604 33.9355 33.7745 29.4872 33.7745 24C33.7745 18.5128 29.3604 14.0645 23.9153 14.0645C18.4702 14.0645 14.0562 18.5128 14.0562 24C14.0562 29.4872 18.4702 33.9355 23.9153 33.9355Z"
          stroke="white"
          strokeLinejoin="round"
        />
        <path
          d="M23.9155 18.4787V24L27.2254 27.3355"
          stroke="white"
          strokeLinejoin="round"
        />
      </g>
      <defs>
        <clipPath id={id}>
          <rect width="22" height="22" fill="white" transform="translate(13 13)" />
        </clipPath>
      </defs>
    </svg>
  );
};
