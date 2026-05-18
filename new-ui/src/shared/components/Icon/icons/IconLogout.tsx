import type { SVGProps } from 'react';

export const IconLogout = (props: SVGProps<SVGSVGElement>) => {
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
        d="M17.75 4.75V14.75C17.75 16.27 16.52 17.5 15 17.5H13V16H15C15.69 16 16.25 15.44 16.25 14.75V4.75C16.25 4.06 15.69 3.5 15 3.5H13V2H15C16.52 2 17.75 3.23 17.75 4.75ZM12.98 9.22L8.89 5.13L7.83 6.19L10.64 8.99H3V10.49H10.64L7.83 13.29L8.89 14.35L12.98 10.27C13.12 10.13 13.2 9.94 13.2 9.74C13.2 9.54 13.12 9.35 12.98 9.21V9.22Z"
        fill="#7E8794"
      />
    </svg>
  );
};
