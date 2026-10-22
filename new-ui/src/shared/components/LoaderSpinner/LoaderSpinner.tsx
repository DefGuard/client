import './style.scss';
import clsx from 'clsx';

type Props = {
  size?: number;
  thickness?: number;
  variant?: 'empty' | 'primary';
};

const outerRadiusRatio = 0.4;
const indicatorArcRatio = 0.265;
const indicatorStartDeg = 62;

export const LoaderSpinner = ({ size = 20, thickness = 2, variant }: Props) => {
  const center = size / 2;
  const radius = size * outerRadiusRatio - thickness / 2;
  const circumference = 2 * Math.PI * radius;
  const arc = circumference * indicatorArcRatio;

  return (
    <div
      className={clsx('loader-spinner', variant && `variant-${variant}`)}
      style={{
        height: size,
        width: size,
      }}
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        fill="none"
      >
        <circle
          className="track"
          cx={center}
          cy={center}
          r={radius}
          strokeWidth={thickness}
        />
        <circle
          className="indicator"
          cx={center}
          cy={center}
          r={radius}
          strokeWidth={thickness}
          strokeDasharray={`${arc} ${circumference}`}
          transform={`rotate(${indicatorStartDeg} ${center} ${center})`}
        />
      </svg>
    </div>
  );
};
