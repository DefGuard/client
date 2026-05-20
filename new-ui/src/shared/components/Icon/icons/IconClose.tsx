import type { SVGProps } from 'react';

export const IconClose = (props: SVGProps<SVGSVGElement>) => {
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
        d="M15 6.16998L13.83 5L10 8.83002L6.16998 5L5 6.16998L8.83002 10L5 13.83L6.16998 15L10 11.17L13.83 15L15 13.83L11.17 10L15 6.16998Z"
        fill="#7E8794"
      />
    </svg>
  );
};
