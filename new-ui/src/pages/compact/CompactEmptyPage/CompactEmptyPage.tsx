import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonSize, ButtonVariant } from '../../../shared/components/Button/types';
import { Icon, IconKind } from '../../../shared/components/Icon';
import { WindowHeader } from '../../../shared/components/WindowHeader/WindowHeader';
import { api } from '../../../shared/rust-api/api';
import {
  getInstancesQueryOptions,
  getTunnelsQueryOptions,
} from '../../../shared/rust-api/query';
import { CompactPage } from '../CompactPage/CompactPage';

export const CompactEmptyPage = () => {
  const navigate = useNavigate();

  const { data: instances } = useQuery(getInstancesQueryOptions);
  const { data: tunnels } = useQuery(getTunnelsQueryOptions);

  const hasAny = (instances?.length ?? 0) > 0 || (tunnels?.length ?? 0) > 0;

  useEffect(() => {
    if (hasAny) {
      void navigate({ to: '/compact', replace: true });
    }
  }, [hasAny, navigate]);

  return (
    <CompactPage containerProps={{ id: 'compact-empty-page' }}>
      <WindowHeader variant="compact" />
      <div className="empty-card">
        <div className="content">
          <Icon icon={IconKind.DisconnectAll} size={26} />
          <p>{`You don't have any instances or tunnels yet. Click the button below to open Defguard.`}</p>
          <Button
            text="Open Defguard"
            variant={ButtonVariant.Primary}
            size={ButtonSize.Primary}
            onClick={() => {
              void api.swapToFullView();
            }}
          />
        </div>
      </div>
    </CompactPage>
  );
};
