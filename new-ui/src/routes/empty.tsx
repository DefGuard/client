import { useQuery } from '@tanstack/react-query';
import { createFileRoute, useNavigate } from '@tanstack/react-router';
import { useEffect } from 'react';

import { hasAnyVisibleLocationsQueryOptions } from '../shared/rust-api/query';

export const Route = createFileRoute('/empty')({
  component: RouteComponent,
});

function RouteComponent() {
  const navigate = useNavigate();
  const { data: hasLocations } = useQuery(hasAnyVisibleLocationsQueryOptions);

  useEffect(() => {
    if (hasLocations === true) {
      void navigate({ to: '/' });
    }
  }, [hasLocations, navigate]);

  return <div></div>;
}
