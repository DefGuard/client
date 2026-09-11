import clsx from 'clsx';
import { Icon } from '../Icon';
import './style.scss';
import { useMemo } from 'react';

type Props = {
  size?: number;
  variant?: 'empty' | 'primary';
};

export const LoaderSpinner = ({ size = 20, variant }: Props) => {
  const variantClass = useMemo(() => (variant ? `variant-${variant}` : null), [variant]);
  return (
    <div
      className={clsx('loader-spinner', variantClass)}
      style={{
        height: size,
        width: size,
      }}
    >
      <Icon icon="loader" size={size} />
    </div>
  );
};
