import { useId } from 'react';

type Props = {
  height?: number;
  width?: number;
  id?: string;
  className?: string;
};

export const IconDefguard = ({ id, className, height = 43, width = 21 }: Props) => {
  const clipPathId = useId();
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={width}
      height={height}
      fill="none"
      viewBox="0 0 21 43"
      className={className}
      id={id}
    >
      <g clipPath={`url(#${clipPathId})`}>
        <path
          fill="#0C8CE0"
          d="M18.706 0v9.315l-8.178-4.658L0 10.65v22.625l10.52 5.992 8.178-4.658v4.8l-3.957 2.258L17.08 43l3.957-2.257V21.29L10.52 15.298 2.34 19.956v-7.98l8.18-4.658 8.178 4.657v2.652l2.34 1.334V1.334L18.698 0zM2.34 31.94v-7.981l8.18 4.657 8.178-4.657v7.98l-8.179 4.658zm15.192-9.978-7.013 3.994-7.013-3.994 7.013-3.995z"
        />
      </g>
      <defs>
        <clipPath id={clipPathId}>
          <path fill="#fff" d="M0 0h21v43H0z" />
        </clipPath>
      </defs>
    </svg>
  );
};
