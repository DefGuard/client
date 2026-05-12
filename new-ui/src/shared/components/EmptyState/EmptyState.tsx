import { useMemo } from 'react';
import './style.scss';
import clsx from 'clsx';
import { ThemeSpacing } from '../../types';
import { isPresent } from '../../utils/isPresent';
import { Button } from '../Button/Button';
import { SizedBox } from '../SizedBox/SizedBox';
import type { EmptyStateProps } from './types';

const Empty = () => {
  return null;
};

export const EmptyState = ({
  ref,
  icon,
  primaryAction,
  secondaryAction,
  secondaryActionText,
  subtitle,
  title,
  className,
  id,
  testId,
}: EmptyStateProps) => {
  const RenderIcon = useMemo(() => {
    if (!icon) return Empty;
    return Empty;
  }, [icon]);

  return (
    <div
      ref={ref}
      className={clsx('empty-state', className)}
      id={id}
      data-testid={testId}
    >
      {isPresent(icon) && (
        <>
          <RenderIcon />
          <SizedBox height={ThemeSpacing.Lg} />
        </>
      )}
      {isPresent(title) && (
        <>
          <p className="title">{title}</p>
          <SizedBox height={4} />
        </>
      )}
      {isPresent(subtitle) && <p className="subtitle">{subtitle}</p>}
      <SizedBox height={ThemeSpacing.Lg} />
      {isPresent(primaryAction) && (
        <>
          <Button {...primaryAction} />
          <SizedBox height={ThemeSpacing.Lg} />
        </>
      )}
      {isPresent(secondaryAction) && isPresent(secondaryActionText) && (
        <button className="secondary-action" onClick={secondaryAction}>
          {secondaryActionText}
        </button>
      )}
    </div>
  );
};
