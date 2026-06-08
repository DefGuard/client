import { useNavigate } from '@tanstack/react-router';
import { useId } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';

export const SessionTimeoutPage = () => {
  const navigate = useNavigate();
  return (
    <div id="session-timeout">
      <div className="positioner">
        <ClockIcon />
        <p className="title">{`Session time out`}</p>
        <p className="description">{`Sorry, you have exceeded the time limit to complete the process. 
Please try again. If you need assistance, please watch our guide 
or contact your administrator.`}</p>
        <Button
          text="Enter new token"
          variant={ButtonVariant.Primary}
          onClick={() => {
            navigate({ to: '/full/add/instance', replace: true });
          }}
        />
      </div>
    </div>
  );
};

const ClockIcon = () => {
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
        stroke-opacity="0.4"
        stroke-dasharray="2 2"
      />
      <g clip-path={`"url(#${id})"`}>
        <path
          d="M25.2447 35.9689C31.1848 35.9689 36.0001 31.1163 36.0001 25.1302C36.0001 19.1442 31.1848 14.2915 25.2447 14.2915C19.3046 14.2915 14.4893 19.1442 14.4893 25.1302C14.4893 31.1163 19.3046 35.9689 25.2447 35.9689Z"
          fill="white"
          fill-opacity="0.1"
        />
        <path
          d="M23.9078 34.8388C29.8479 34.8388 34.6632 29.9861 34.6632 24.0001C34.6632 18.014 29.8479 13.1614 23.9078 13.1614C17.9677 13.1614 13.1523 18.014 13.1523 24.0001C13.1523 29.9861 17.9677 34.8388 23.9078 34.8388Z"
          stroke="white"
          stroke-linejoin="round"
        />
        <path
          d="M23.9077 17.9768V24L27.5185 27.6387"
          stroke="white"
          stroke-linejoin="round"
        />
      </g>
      <defs>
        <clipPath id={id}>
          <rect width="24" height="24" fill="white" transform="translate(12 12)" />
        </clipPath>
      </defs>
    </svg>
  );
};
