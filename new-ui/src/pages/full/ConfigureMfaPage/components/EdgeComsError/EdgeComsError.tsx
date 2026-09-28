import './style.scss';
import type { PropsWithChildren } from 'react';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { EmptyState } from '../../../../../shared/components/EmptyState/EmptyState';
import { EmptyIcon } from '../../../../../shared/components/EmptyState/types';
import { dismissEdgeComsError, useEdgeComsErrorStore } from './useEdgeComsErrorStore';

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
          onClick: dismissEdgeComsError,
        }}
      />
    </div>
  );
};

/** Hides the caller rather than unmounting it, so dismissing returns to it as it was. */
export const EdgeComsErrorHost = ({ children }: PropsWithChildren) => {
  const visible = useEdgeComsErrorStore((s) => s.visible);

  return (
    <>
      {visible && <EdgeComsError />}
      <div style={{ display: visible ? 'none' : 'contents' }}>{children}</div>
    </>
  );
};
