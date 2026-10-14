import { useQuery } from '@tanstack/react-query';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useEffect, useMemo } from 'react';

import { WindowId } from '../shared/consts';
import {
  getInstancesQueryOptions,
  getTunnelsQueryOptions,
} from '../shared/rust-api/query';

export const Route = createFileRoute('/empty')({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const { data: instances } = useQuery(getInstancesQueryOptions);
  const { data: tunnels } = useQuery(getTunnelsQueryOptions);

  const hasContent = useMemo(
    () => (instances?.length ?? 0) > 0 || (tunnels?.length ?? 0) > 0,
    [instances, tunnels],
  );

  useEffect(() => {
    if (!hasContent) return;
    const to = getCurrentWindow().label === WindowId.CompactView ? '/compact' : '/full';
    void navigate({ to, replace: true });
  }, [hasContent, navigate]);

  return <div></div>;
}
