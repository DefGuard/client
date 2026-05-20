import type { SVGProps } from 'react';

export const IconPieChart = (props: SVGProps<SVGSVGElement>) => {
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
        d="M9.75 2C5.48 2 2 5.48 2 9.75C2 14.02 5.48 17.5 9.75 17.5C14.02 17.5 17.5 14.02 17.5 9.75C17.5 5.48 14.02 2 9.75 2ZM16 9.75C16 10.4 15.9 11.03 15.72 11.62L10.5 9.27V3.55C13.59 3.92 16 6.56 16 9.75ZM3.5 9.75C3.5 6.56 5.91 3.92 9 3.55V9.75C9 9.86 9.03 9.96 9.08 10.06L11.68 15.69C11.07 15.89 10.43 16 9.76 16C6.31 16 3.51 13.2 3.51 9.75H3.5ZM13.03 15.06L11.27 11.26L15.09 12.98C14.58 13.83 13.87 14.54 13.03 15.06Z"
        fill="#7E8794"
      />
    </svg>
  );
};
