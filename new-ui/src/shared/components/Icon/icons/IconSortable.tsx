import type { SVGProps } from 'react';

export const IconSortable = (props: SVGProps<SVGSVGElement>) => {
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
        d="M9.39 8.97L6.99 6.57V15.65H5.49V6.53L3.06 8.96L2 7.9L5.69 4.21C5.97 3.93 6.47 3.93 6.75 4.21L10.45 7.9L9.39 8.96V8.97ZM17.13 11.29L14.7 13.72V4.61H13.2V13.69L10.8 11.29L9.74 12.35L13.43 16.04C13.57 16.18 13.76 16.26 13.96 16.26C14.16 16.26 14.35 16.18 14.49 16.04L18.18 12.35L17.12 11.29H17.13Z"
        fill="#7E8794"
      />
    </svg>
  );
};
