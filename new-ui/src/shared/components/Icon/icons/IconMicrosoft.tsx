import type { SVGProps } from 'react';

export const IconMicrosoft = (props: SVGProps<SVGSVGElement>) => {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      className="color-icon"
      width="20"
      height="20"
      viewBox="0 0 20 20"
      fill="none"
      {...props}
    >
      <rect x="2" y="2" width="7.52941" height="7.52941" fill="#F35325" />
      <rect x="10.4706" y="2" width="7.52941" height="7.52941" fill="#81BC06" />
      <rect x="2" y="10.4706" width="7.52941" height="7.52941" fill="#05A6F0" />
      <rect x="10.4706" y="10.4706" width="7.52941" height="7.52941" fill="#FFBA08" />
    </svg>
  );
};
