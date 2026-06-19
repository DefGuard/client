import type { CSSProperties, PropsWithChildren } from 'react';
import { ThemeSpacing, type ThemeSpacingValue } from '../../types';

type Props = PropsWithChildren<{
  split?: number;
  spacing?: ThemeSpacingValue;
}>;

export const Split = ({ children, split = 2, spacing = ThemeSpacing.Sm }: Props) => {
  const style: CSSProperties = {
    display: 'grid',
    gridTemplateColumns: `repeat(${split}, 1fr)`,
    columnGap: spacing,
    width: '100%',
  };

  return (
    <div className="split-component" style={style}>
      {children}
    </div>
  );
};
