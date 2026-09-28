import './style.scss';
import { useRouterState } from '@tanstack/react-router';
import { type PropsWithChildren, useEffect } from 'react';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { EmptyState } from '../../../../../shared/components/EmptyState/EmptyState';
import { EmptyIcon } from '../../../../../shared/components/EmptyState/types';
import {
  dismissEdgeComsError,
  retryEdgeComsError,
  useEdgeComsErrorStore,
} from './useEdgeComsErrorStore';

const DESCRIPTION = `An unexpected error occurred while processing your request. Please try again later.`;

export const EdgeComsError = () => {
  return (
    <div id="configure-mfa-edge-coms-error">
      <EmptyState
        title="Service Unavailable"
        subtitle={DESCRIPTION}
        icon={EmptyIcon.ServiceUnavailable}
        primaryAction={{
          text: 'Refresh',
          variant: ButtonVariant.Primary,
          onClick: retryEdgeComsError,
        }}
      />
    </div>
  );
};

/** Hides the caller rather than unmounting it, so dismissing returns to it as it was. */
export const EdgeComsErrorHost = ({ children }: PropsWithChildren) => {
  const visible = useEdgeComsErrorStore((s) => s.visible);
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  // biome-ignore lint/correctness/useExhaustiveDependencies: fires on route change only
  useEffect(() => {
    dismissEdgeComsError();
  }, [pathname]);

  return (
    <>
      {visible && <EdgeComsError />}
      <div style={{ display: visible ? 'none' : 'contents' }}>{children}</div>
    </>
  );
};
