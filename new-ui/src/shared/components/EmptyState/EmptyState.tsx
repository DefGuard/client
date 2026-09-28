import './style.scss';
import clsx from 'clsx';
import { ThemeSpacing } from '../../types';
import { isPresent } from '../../utils/isPresent';
import { Button } from '../Button/Button';
import { SizedBox } from '../SizedBox/SizedBox';
import { EmptyStateServiceUnavailable } from './components/EmptyStateIcon/icons/EmptyStateServiceUnavailable';
import { EmptyStateSessionTimeout } from './components/EmptyStateIcon/icons/EmptyStateSessionTimeout';
import type { EmptyStateProps } from './types';

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
  const renderIcon = () => {
    if (!isPresent(icon)) return null;
    switch (icon) {
      case 'session-timeout':
        return <EmptyStateSessionTimeout />;
      case 'service-unavailable':
        return <EmptyStateServiceUnavailable />;
    }
  };

  return (
    <div
      ref={ref}
      className={clsx('empty-state', className)}
      id={id}
      data-testid={testId}
    >
      {isPresent(icon) && (
        <>
          {renderIcon()}
          <SizedBox height={ThemeSpacing.Sm} />
        </>
      )}
      {isPresent(title) && <p className="title">{title}</p>}
      {isPresent(subtitle) && (
        <>
          <SizedBox height={4} />
          <p className="subtitle">{subtitle}</p>
        </>
      )}
      {isPresent(primaryAction) && (
        <>
          <SizedBox height={ThemeSpacing.Lg} />
          <Button {...primaryAction} />
        </>
      )}
      {isPresent(secondaryAction) && isPresent(secondaryActionText) && (
        <>
          <SizedBox height={ThemeSpacing.Lg} />
          <button className="secondary-action" onClick={secondaryAction}>
            {secondaryActionText}
          </button>
        </>
      )}
    </div>
  );
};
