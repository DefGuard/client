import type { SVGProps } from 'react';
const SvgIconCheckmarkSmall = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={18}
    height={18}
    fill="none"
    viewBox="0 0 18 18"
    {...props}
  >
    <g fill="#fff" mask="url(#icon-checkmark-small_svg__mask0)">
      <path d="m8.418 11.828 5.85-5.85a.828.828 0 0 0-1.17-1.17l-5.85 5.85a.827.827 0 1 0 1.17 1.17" />
      <path d="M8.37 10.7 4.86 7.19a.827.827 0 1 0-1.17 1.17l3.51 3.51a.827.827 0 1 0 1.17-1.17" />
    </g>
  </svg>
);
export default SvgIconCheckmarkSmall;
