import './style.scss';
import clsx from 'clsx';
import { isPresent } from '../../utils/isPresent';
import { InteractionBox } from '../InteractionBox/InteractionBox';
import type { FieldBoxProps } from './types';

// generalized field box for components like Input, shouldn't be in layout on it's own
export const FieldBox = ({
  children,
  disabled,
  error,
  className,
  boxRef,
  interactionRef,
  iconLeft,
  iconRight,
  size,
  forceFocusState,
  onInteractionClick,
  reserveInteraction = false,
  ...rest
}: FieldBoxProps) => {
  const hasIconLeft = isPresent(iconLeft);
  const hasIconRight = isPresent(iconRight) || reserveInteraction;
  return (
    <div
      className={clsx('field-box', className, `size-${size}`, {
        'grid-default': !hasIconLeft && !hasIconRight,
        'grid-left': hasIconLeft && !hasIconRight,
        'grid-right': hasIconRight && !hasIconLeft,
        'grid-both': hasIconLeft && hasIconRight,
        focus: forceFocusState,
        disabled,
        error,
      })}
      ref={boxRef}
      {...rest}
    >
      {hasIconLeft && iconLeft}
      {children}
      {hasIconRight && (
        <>
          {isPresent(iconRight) && (
            <InteractionBox
              disabled={disabled}
              onClick={onInteractionClick}
              tabIndex={-1}
              ref={interactionRef}
            >
              {iconRight}
            </InteractionBox>
          )}
          {!isPresent(iconRight) && <div className="empty"></div>}
        </>
      )}
    </div>
  );
};
