import type { SVGProps } from 'react';
const SvgDefguardLogoIcon = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={21}
    height={44}
    fill="none"
    viewBox="0 0 21 44"
    {...props}
  >
    <path
      fill="url(#defguard-logo-icon_svg__a)"
      d="M20.89 17.962V2.001L17.413 0v7.965L10.459 3.96.027 9.966v24.018l10.432 6.004 6.954-4.003V40l-3.473 1.998L17.417 44l3.473-1.998V21.968l-10.431-5.998-6.955 4.003v-8.01L10.46 7.96l6.954 4.003v3.998l3.477 2.001Zm-10.431 2.013 3.477 2-3.477 2.002-3.477-2.001 3.477-2.001Zm0 8.004 6.954-4.002v8.006l-6.954 4.003-6.955-4.003v-8.006l6.955 4.002Z"
    />
    <defs>
      <linearGradient
        id="defguard-logo-icon_svg__a"
        x1={10.459}
        x2={10.459}
        y1={0}
        y2={44}
        gradientUnits="userSpaceOnUse"
      >
        <stop stopColor="#2ACCFF" />
        <stop offset={1} stopColor="#0071D4" />
      </linearGradient>
    </defs>
  </svg>
);
export default SvgDefguardLogoIcon;
