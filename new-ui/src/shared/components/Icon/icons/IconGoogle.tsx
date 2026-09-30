import { type SVGProps, useId } from 'react';

export const IconGoogle = (props: SVGProps<SVGSVGElement>) => {
  const id = useId();

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
      <g clipPath={`url(#${id}-clip0_4_22)`}>
        <mask
          id={`${id}-mask0_4_22`}
          style={{ maskType: 'luminance' }}
          maskUnits="userSpaceOnUse"
          x="0"
          y="0"
          width="20"
          height="20"
        >
          <path
            d="M19.8079 8.14476H10.213V11.9894H15.7263C15.6377 12.5336 15.4387 13.0688 15.1472 13.5569C14.8133 14.1161 14.4006 14.5418 13.9775 14.866C12.7101 15.8372 11.2325 16.0358 10.2063 16.0358C7.61409 16.0358 5.39919 14.3604 4.54176 12.0838C4.50716 12.0012 4.48418 11.9159 4.4562 11.8315C4.26673 11.2521 4.1632 10.6384 4.1632 10.0006C4.1632 9.33685 4.27531 8.70145 4.47972 8.10134C5.28599 5.73451 7.55082 3.96672 10.2082 3.96672C10.7427 3.96672 11.2574 4.03034 11.7455 4.15724C12.861 4.44725 13.6501 5.01843 14.1336 5.47023L17.0511 2.61313C15.2764 0.985972 12.9629 2.46015e-09 10.2033 2.46015e-09C7.99721 -4.74827e-05 5.96042 0.687318 4.29133 1.84899C2.93776 2.79107 1.82764 4.0524 1.07844 5.5173C0.381576 6.87555 0 8.38075 0 9.99913C0 11.6176 0.382159 13.1384 1.07902 14.4841V14.4932C1.81508 15.9218 2.89146 17.1519 4.19967 18.0897C5.34254 18.9089 7.39181 20 10.2033 20C11.8202 20 13.2531 19.7085 14.5169 19.1622C15.4285 18.7681 16.2363 18.2541 16.9676 17.5935C17.9339 16.7206 18.6906 15.6409 19.2072 14.3987C19.7237 13.1565 20 11.7518 20 10.2289C20 9.51963 19.9288 8.79934 19.8079 8.14469V8.14476Z"
            fill="white"
          />
        </mask>
        <g mask={`url(#${id}-mask0_4_22)`}>
          <g filter={`url(#${id}-filter0_f_4_22)`}>
            <path
              d="M-0.147156 10.067C-0.13655 11.6599 0.317344 13.3034 1.00439 14.6302V14.6393C1.50081 15.6028 2.17928 16.3639 2.95204 17.118L7.61933 15.415C6.73631 14.9665 6.60157 14.6916 5.96858 14.1901C5.32173 13.5378 4.83963 12.789 4.53939 11.911H4.52729L4.53939 11.9019C4.34187 11.3221 4.32238 10.7066 4.3151 10.067H-0.147156Z"
              fill={`url(#${id}-paint0_radial_4_22)`}
            />
          </g>
          <g filter={`url(#${id}-filter1_f_4_22)`}>
            <path
              d="M10.2131 -0.0727539C9.75178 1.54791 9.92816 3.12324 10.2131 4.03984C10.7458 4.04024 11.259 4.10374 11.7456 4.23024C12.8611 4.52026 13.6501 5.09145 14.1336 5.54325L17.1257 2.61316C15.3532 0.987939 13.22 -0.0701933 10.2131 -0.0727539Z"
              fill={`url(#${id}-paint1_radial_4_22)`}
            />
          </g>
          <g filter={`url(#${id}-filter2_f_4_22)`}>
            <path
              d="M10.2031 -0.0855713C7.94031 -0.0856202 5.85124 0.619392 4.13931 1.81089C3.50367 2.25329 2.92036 2.76434 2.40095 3.33264C2.26488 4.60919 3.41955 6.1782 5.70615 6.16521C6.8156 4.87467 8.45645 4.0397 10.2827 4.0397C10.2844 4.0397 10.286 4.03983 10.2877 4.03984L10.2131 -0.0852793C10.2097 -0.0852814 10.2064 -0.0855713 10.2031 -0.0855713Z"
              fill={`url(#${id}-paint2_radial_4_22)`}
            />
          </g>
          <g filter={`url(#${id}-filter3_f_4_22)`}>
            <path
              d="M17.6715 10.5289L15.6518 11.9164C15.5632 12.4605 15.364 12.9958 15.0726 13.4839C14.7387 14.0431 14.3259 14.4688 13.9028 14.793C12.6381 15.7622 11.1644 15.9617 10.1385 15.9625C9.07813 17.7685 8.89223 18.6731 10.2131 20.1307C11.8475 20.1295 13.2965 19.8345 14.5746 19.282C15.4985 18.8826 16.317 18.3617 17.0581 17.6922C18.0374 16.8077 18.8044 15.7135 19.3279 14.4546C19.8513 13.1957 20.1312 11.7723 20.1312 10.2289L17.6715 10.5289Z"
              fill={`url(#${id}-paint3_radial_4_22)`}
            />
          </g>
          <g filter={`url(#${id}-filter4_f_4_22)`}>
            <path
              d="M10.0639 7.99866V12.1355H19.7811C19.8665 11.5689 20.1492 10.8358 20.1492 10.2289C20.1492 9.51962 20.078 8.65331 19.9572 7.99866H10.0639Z"
              fill="#3086FF"
            />
          </g>
          <g filter={`url(#${id}-filter5_f_4_22)`}>
            <path
              d="M2.44732 3.18665C1.84767 3.84274 1.33539 4.5771 0.929209 5.37129C0.232358 6.72955 -0.1492 8.38081 -0.1492 9.99919C-0.1492 10.022 -0.147313 10.0443 -0.147161 10.0671C0.161453 10.6588 4.11578 10.5455 4.31509 10.0671C4.31484 10.0448 4.31233 10.023 4.31233 10.0006C4.31233 9.33683 4.42447 8.84758 4.62888 8.24746C4.88104 7.50723 5.27587 6.82559 5.78075 6.23829C5.8952 6.09217 6.20048 5.77805 6.28955 5.58963C6.32348 5.51787 6.22795 5.47758 6.22261 5.45232C6.21664 5.42407 6.08856 5.44679 6.05987 5.42574C5.96877 5.35892 5.78837 5.32403 5.67882 5.29301C5.44468 5.22671 5.05663 5.08051 4.8411 4.92895C4.15981 4.44988 3.0966 3.87764 2.44732 3.18665Z"
              fill={`url(#${id}-paint4_radial_4_22)`}
            />
          </g>
          <g filter={`url(#${id}-filter6_f_4_22)`}>
            <path
              d="M4.85573 5.45513C6.43557 6.41212 6.8899 4.97208 7.94027 4.52146L6.11312 0.732422C5.44099 1.01492 4.80596 1.36589 4.21675 1.77597C3.33682 2.3884 2.55977 3.13574 1.91788 3.98622L4.85573 5.45513Z"
              fill={`url(#${id}-paint5_radial_4_22)`}
            />
          </g>
          <g filter={`url(#${id}-filter7_f_4_22)`}>
            <path
              d="M5.49872 15.1223C3.37799 15.8879 3.04598 15.9153 2.85077 17.2296C3.22381 17.5937 3.62462 17.9304 4.05054 18.2357C5.1934 19.0549 7.39179 20.146 10.2033 20.146C10.2066 20.146 10.2098 20.1457 10.2131 20.1457V15.8895C10.211 15.8895 10.2085 15.8897 10.2064 15.8897C9.15355 15.8897 8.31225 15.6131 7.44964 15.1323C7.23696 15.0137 6.8511 15.3321 6.65495 15.1897C6.38442 14.9934 5.73336 15.3589 5.49872 15.1223Z"
              fill={`url(#${id}-paint6_radial_4_22)`}
            />
          </g>
          <g opacity="0.5" filter={`url(#${id}-filter8_f_4_22)`}>
            <path
              d="M8.9711 15.7555V20.072C9.36448 20.1181 9.77388 20.1461 10.2033 20.1461C10.6339 20.1461 11.0504 20.1239 11.4551 20.0833V15.7846C11.0016 15.8621 10.5744 15.8897 10.2064 15.8897C9.7826 15.8897 9.37045 15.8404 8.9711 15.7555Z"
              fill={`url(#${id}-paint7_linear_4_22)`}
            />
          </g>
        </g>
      </g>
      <defs>
        <filter
          id={`${id}-filter0_f_4_22`}
          x="-0.617235"
          y="9.59694"
          width="8.70664"
          height="7.99118"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter1_f_4_22`}
          x="9.45936"
          y="-0.542833"
          width="8.13642"
          height="6.55613"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter2_f_4_22`}
          x="1.92009"
          y="-0.555651"
          width="8.83765"
          height="7.19101"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter3_f_4_22`}
          x="8.81227"
          y="9.7588"
          width="11.7891"
          height="10.842"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter4_f_4_22`}
          x="9.59385"
          y="7.52858"
          width="11.0254"
          height="5.077"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter5_f_4_22`}
          x="-0.61928"
          y="2.71657"
          width="7.38617"
          height="8.223"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter6_f_4_22`}
          x="-1.38697"
          y="-2.57243"
          width="12.6321"
          height="11.654"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="1.65243" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter7_f_4_22`}
          x="2.38069"
          y="14.6351"
          width="8.30246"
          height="5.98093"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <filter
          id={`${id}-filter8_f_4_22`}
          x="8.50102"
          y="15.2854"
          width="3.42417"
          height="5.33066"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend
            mode="normal"
            in="SourceGraphic"
            in2="BackgroundImageFix"
            result="shape"
          />
          <feGaussianBlur stdDeviation="0.23504" result="effect1_foregroundBlur_4_22" />
        </filter>
        <radialGradient
          id={`${id}-paint0_radial_4_22`}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(-0.415601 -9.95993 14.9426 -0.597686 7.5259 16.9679)"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.141612" stopColor="#1ABD4D" />
          <stop offset="0.247515" stopColor="#6EC30D" />
          <stop offset="0.311547" stopColor="#8AC502" />
          <stop offset="0.366013" stopColor="#A2C600" />
          <stop offset="0.445673" stopColor="#C8C903" />
          <stop offset="0.540305" stopColor="#EBCB03" />
          <stop offset="0.615636" stopColor="#F7CD07" />
          <stop offset="0.699345" stopColor="#FDCD04" />
          <stop offset="0.771242" stopColor="#FDCE05" />
          <stop offset="0.860566" stopColor="#FFCE0A" />
        </radialGradient>
        <radialGradient
          id={`${id}-paint1_radial_4_22`}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(7.05806 -1.69631e-05 -9.92038e-06 8.92438 16.8458 5.33125)"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.408458" stopColor="#FB4E5A" />
          <stop offset="1" stopColor="#FF4540" />
        </radialGradient>
        <radialGradient
          id={`${id}-paint2_radial_4_22`}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(-9.88885 5.36243 7.4323 13.1383 12.9913 -1.37741)"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.231273" stopColor="#FF4541" />
          <stop offset="0.311547" stopColor="#FF4540" />
          <stop offset="0.457516" stopColor="#FF4640" />
          <stop offset="0.540305" stopColor="#FF473F" />
          <stop offset="0.699346" stopColor="#FF5138" />
          <stop offset="0.771242" stopColor="#FF5B33" />
          <stop offset="0.860566" stopColor="#FF6C29" />
          <stop offset="1" stopColor="#FF8C18" />
        </radialGradient>
        <radialGradient
          id={`${id}-paint3_radial_4_22`}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(-17.9337 -22.9206 -8.64137 6.48127 10.3601 18.8363)"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.131546" stopColor="#0CBA65" />
          <stop offset="0.209784" stopColor="#0BB86D" />
          <stop offset="0.297297" stopColor="#09B479" />
          <stop offset="0.396257" stopColor="#08AD93" />
          <stop offset="0.477124" stopColor="#0AA6A9" />
          <stop offset="0.568425" stopColor="#0D9CC6" />
          <stop offset="0.667385" stopColor="#1893DD" />
          <stop offset="0.768727" stopColor="#258BF1" />
          <stop offset="0.858506" stopColor="#3086FF" />
        </radialGradient>
        <radialGradient
          id={`${id}-paint4_radial_4_22`}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(-1.26913 10.7101 15.1251 1.71807 9.33671 1.80342)"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.366013" stopColor="#FF4E3A" />
          <stop offset="0.457516" stopColor="#FF8A1B" />
          <stop offset="0.540305" stopColor="#FFA312" />
          <stop offset="0.615636" stopColor="#FFB60C" />
          <stop offset="0.771242" stopColor="#FFCD0A" />
          <stop offset="0.860566" stopColor="#FECF0A" />
          <stop offset="0.915033" stopColor="#FECF08" />
          <stop offset="1" stopColor="#FDCD01" />
        </radialGradient>
        <radialGradient
          id={`${id}-paint5_radial_4_22`}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(-3.66844 3.97231 -11.4435 -10.1305 7.55198 1.6923)"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.315904" stopColor="#FF4C3C" />
          <stop offset="0.603818" stopColor="#FF692C" />
          <stop offset="0.726837" stopColor="#FF7825" />
          <stop offset="0.884534" stopColor="#FF8D1B" />
          <stop offset="1" stopColor="#FF9F13" />
        </radialGradient>
        <radialGradient
          id={`${id}-paint6_radial_4_22`}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(-9.88885 -5.36243 7.4323 -13.1383 12.9913 21.3771)"
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0.231273" stopColor="#0FBC5F" />
          <stop offset="0.311547" stopColor="#0FBC5F" />
          <stop offset="0.366013" stopColor="#0FBC5E" />
          <stop offset="0.457516" stopColor="#0FBC5D" />
          <stop offset="0.540305" stopColor="#12BC58" />
          <stop offset="0.699346" stopColor="#28BF3C" />
          <stop offset="0.771242" stopColor="#38C02B" />
          <stop offset="0.860566" stopColor="#52C218" />
          <stop offset="0.915033" stopColor="#67C30F" />
          <stop offset="1" stopColor="#86C504" />
        </radialGradient>
        <linearGradient
          id={`${id}-paint7_linear_4_22`}
          x1="8.9711"
          y1="17.9508"
          x2="11.4551"
          y2="17.9508"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#0FBC5C" />
          <stop offset="1" stopColor="#0CBA65" />
        </linearGradient>
        <clipPath id={`${id}-clip0_4_22`}>
          <rect width="20" height="20" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
};
