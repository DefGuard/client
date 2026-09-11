import type { SVGProps } from 'react';

export const IconMenu = (props: SVGProps<SVGSVGElement>) => {
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
        d="M17 9.56C17 10.42 16.3 11.12 15.44 11.12C14.58 11.12 13.88 10.42 13.88 9.56C13.88 8.7 14.58 8 15.44 8C16.3 8 17 8.7 17 9.56ZM10 8C9.14 8 8.44 8.7 8.44 9.56C8.44 10.42 9.14 11.12 10 11.12C10.86 11.12 11.56 10.42 11.56 9.56C11.56 8.7 10.86 8 10 8ZM4.56 8C3.7 8 3 8.7 3 9.56C3 10.42 3.7 11.12 4.56 11.12C5.42 11.12 6.12 10.42 6.12 9.56C6.12 8.7 5.42 8 4.56 8Z"
        fill="#7E8794"
      />
    </svg>
  );
};
