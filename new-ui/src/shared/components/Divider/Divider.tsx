import './style.scss';
import clsx from 'clsx';
import { type CSSProperties, useMemo } from 'react';
import type { OrientationValue, ThemeSpacingValue } from '../../types';
import { isPresent } from '../../utils/isPresent';

type Props = {
  text?: string;
  orientation?: OrientationValue;
  spacing?: ThemeSpacingValue;
};

export const Divider = ({ text, spacing, orientation = 'horizontal' }: Props) => {
  const textPresent = isPresent(text) && text.length > 0;

  const style = useMemo((): CSSProperties => {
    const res: CSSProperties = {};
    if (spacing) {
      switch (orientation) {
        case 'horizontal':
          res.paddingTop = spacing;
          res.paddingBottom = spacing;
          break;
        case 'vertical':
          res.paddingLeft = spacing;
          res.paddingRight = spacing;
          break;
      }
    }
    return res;
  }, [orientation, spacing]);

  return (
    <div
      className={clsx('divider', orientation, {
        text: textPresent,
      })}
      style={style}
    >
      {orientation === 'horizontal' && (
        <>
          {textPresent && (
            <>
              <Line />
              <span>{text}</span>
              <Line />
            </>
          )}
          {!textPresent && <Line />}
        </>
      )}
      {orientation === 'vertical' && <Line />}
    </div>
  );
};

const Line = () => {
  return <div className="line"></div>;
};
