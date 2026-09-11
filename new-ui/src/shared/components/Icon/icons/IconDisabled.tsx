import type { SVGProps } from 'react';

export const IconDisabled = (props: SVGProps<SVGSVGElement>) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="none"
      {...props}
    >
      <path
        d="M9.75 2C5.48 2 2 5.48 2 9.75C2 14.02 5.48 17.5 9.75 17.5C14.02 17.5 17.5 14.02 17.5 9.75C17.5 5.48 14.02 2 9.75 2ZM3.5 9.75C3.5 8.3 4 6.97 4.83 5.91L13.59 14.67C12.53 15.5 11.2 16 9.75 16C6.3 16 3.5 13.2 3.5 9.75ZM14.65 13.62L5.88 4.85C6.95 4.01 8.29 3.5 9.75 3.5C13.2 3.5 16 6.3 16 9.75C16 11.21 15.49 12.55 14.65 13.62Z"
        fill="#7E8794"
      />
    </svg>
  );
};
