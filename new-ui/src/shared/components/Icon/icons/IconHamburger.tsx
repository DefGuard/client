import type { SVGProps } from 'react';

export const IconHamburger = (props: SVGProps<SVGSVGElement>) => {
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
        d="M17 4V5.5H3V4H17ZM3 10.5H17V9H3V10.5ZM3 15.5H17V14H3V15.5Z"
        fill="#7E8794"
      />
    </svg>
  );
};
