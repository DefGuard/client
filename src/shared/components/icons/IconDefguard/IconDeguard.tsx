import { useId } from 'react';

type Props = {
  height?: number;
  width?: number;
  id?: string;
  className?: string;
};

export const IconDefguard = ({ id, className, height = 44, width = 21 }: Props) => {
  const gradientId = useId();
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={width}
      height={height}
      fill="none"
      viewBox="0 0 21 44"
      className={className}
      id={id}
    >
      <path
        fill={`url(#${gradientId})`}
        d="M20.8625 17.9624V2.00098L17.3855 0V7.9646L10.4314 3.96118L0 9.96558V33.9839L10.4314 39.9882L17.3855 35.9849V40.0002L13.9127 41.9985L17.3899 44L20.8625 42.0015V21.969L10.4314 15.9714L3.47705 19.9739V11.9634L10.4314 7.96045L17.3855 11.9634V15.9612L20.8625 17.9624ZM10.4314 19.9749L13.9084 21.9758L10.4314 23.9771L6.95422 21.9758L10.4314 19.9749ZM10.4314 27.9789L17.3855 23.9771V31.983L10.4314 35.9856L3.47705 31.983V23.9771L10.4314 27.9789Z"
      />
      <defs>
        <linearGradient
          id={gradientId}
          x1={10.431}
          x2={10.431}
          y1={0}
          y2={44}
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#2ACCFF" />
          <stop offset={1} stopColor="#0071D4" />
        </linearGradient>
      </defs>
    </svg>
  );
};
