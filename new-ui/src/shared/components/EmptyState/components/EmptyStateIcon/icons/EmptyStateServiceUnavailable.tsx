import { useId } from 'react';

export const EmptyStateServiceUnavailable = () => {
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
          d="M25.4194 35.3549C30.9066 35.3549 35.3549 30.9066 35.3549 25.4194C35.3549 19.9322 30.9066 15.4839 25.4194 15.4839C19.9322 15.4839 15.4839 19.9322 15.4839 25.4194C15.4839 30.9066 19.9322 35.3549 25.4194 35.3549Z"
          fill="white"
          fillOpacity="0.1"
        />
        <path
          d="M23.9999 33.9355C29.4872 33.9355 33.9354 29.4872 33.9354 24C33.9354 18.5128 29.4872 14.0645 23.9999 14.0645C18.5127 14.0645 14.0645 18.5128 14.0645 24C14.0645 29.4872 18.5127 33.9355 23.9999 33.9355Z"
          stroke="white"
          strokeLinejoin="round"
        />
        <path
          d="M17.0737 17.1019L30.7705 30.8129"
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
