import type { SVGProps } from 'react';

export const IconClear = (props: SVGProps<SVGSVGElement>) => {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        d="M10 3C6.13 3 3 6.13 3 10C3 13.87 6.13 17 10 17C13.87 17 17 13.87 17 10C17 6.13 13.87 3 10 3ZM13.31 12.25L12.25 13.31L10 11.06L7.75 13.31L6.69 12.25L8.94 10L6.69 7.75L7.75 6.69L10 8.94L12.25 6.69L13.31 7.75L11.06 10L13.31 12.25Z"
        fill="#7E8794"
      />
    </svg>
  );
};
