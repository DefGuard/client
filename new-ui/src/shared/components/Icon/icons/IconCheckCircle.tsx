import type { SVGProps } from 'react';

export const IconCheckCircle = (props: SVGProps<SVGSVGElement>) => {
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
        d="M9.75 2C5.48 2 2 5.48 2 9.75C2 14.02 5.48 17.5 9.75 17.5C14.02 17.5 17.5 14.02 17.5 9.75C17.5 5.48 14.02 2 9.75 2ZM9.75 16C6.3 16 3.5 13.2 3.5 9.75C3.5 6.3 6.3 3.5 9.75 3.5C13.2 3.5 16 6.3 16 9.75C16 13.2 13.2 16 9.75 16ZM12.88 7.23L13.94 8.29L9.96 12.27C9.82 12.41 9.63 12.49 9.43 12.49C9.23 12.49 9.04 12.41 8.9 12.27L6.62 9.99L7.68 8.93L9.43 10.68L12.88 7.23Z"
        fill="#7E8794"
      />
    </svg>
  );
};
