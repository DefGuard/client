import { useId } from 'react';

export const EmptyStateLocations = () => {
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
          d="M25.2819 15.8347C20.3728 15.8347 17.564 19.5239 17.564 23.8581C17.564 28.8243 23.826 33.3132 25.2819 34.5128C26.7379 33.3261 32.9999 28.8759 32.9999 23.8581C32.9999 19.5239 30.1266 15.8347 25.2819 15.8347Z"
          fill="white"
          fillOpacity="0.1"
        />
        <path
          d="M24.3929 25.1739C25.8801 25.1739 27.0858 23.9669 27.0858 22.4779C27.0858 20.989 25.8801 19.782 24.3929 19.782C22.9056 19.782 21.7 20.989 21.7 22.4779C21.7 23.9669 22.9056 25.1739 24.3929 25.1739Z"
          fill="white"
          fillOpacity="0.1"
        />
        <path
          d="M23.6843 14.4675C18.7752 14.4675 15.9663 18.1567 15.9663 22.4909C15.9663 27.4571 22.2283 31.946 23.6843 33.1457C25.1403 31.9589 31.4022 27.5087 31.4022 22.4909C31.4022 18.1567 28.5289 14.4675 23.6843 14.4675Z"
          strokeLinejoin="round"
          stroke="white"
        />
        <path
          d="M26.3773 21.6525C26.3773 23.1414 25.1716 24.3484 23.6844 24.3484C22.1971 24.3484 20.9915 23.1414 20.9915 21.6525C20.9915 20.1636 22.1971 18.9565 23.6844 18.9565C25.1716 18.9565 26.3773 20.1636 26.3773 21.6525Z"
          strokeLinejoin="round"
          stroke="white"
        />
      </g>
      <defs>
        <clipPath id={id}>
          <rect width="18" height="21" fill="white" transform="translate(15 13.5)" />
        </clipPath>
      </defs>
    </svg>
  );
};
