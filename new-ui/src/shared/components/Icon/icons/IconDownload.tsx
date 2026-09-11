import type { SVGProps } from 'react';

export const IconDownload = (props: SVGProps<SVGSVGElement>) => {
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
        d="M17.5 13V15C17.5 16.52 16.27 17.75 14.75 17.75H4.75C3.23 17.75 2 16.52 2 15V13H3.5V15C3.5 15.69 4.06 16.25 4.75 16.25H14.75C15.44 16.25 16 15.69 16 15V13H17.5ZM9.75 13.2C9.94 13.2 10.13 13.13 10.28 12.98L14.36 8.9L13.3 7.84L10.5 10.64V3H9V10.64L6.2 7.84L5.14 8.9L9.22 12.98C9.37 13.13 9.56 13.2 9.75 13.2Z"
        fill="#7E8794"
      />
    </svg>
  );
};
