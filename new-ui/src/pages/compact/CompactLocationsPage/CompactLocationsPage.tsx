import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useLoaderData } from '@tanstack/react-router';
import { Divider } from '../../../shared/components/Divider/Divider';
import { LocationCard } from '../../../shared/components/LocationCard/LocationCard';
import { WindowHeader } from '../../../shared/components/WindowHeader/WindowHeader';
import { getLocationsQueryOptions } from '../../../shared/rust-api/query';
import { ThemeSpacing } from '../../../shared/types';
import { CompactPage } from '../CompactPage/CompactPage';
import { useCompactLocationStore } from './hooks/useCompactLocationsStore';

export const CompactLocationsPage = () => {
  const selectedInstance = useCompactLocationStore((s) => s.selectedInstance);
  const routeData = useLoaderData({ from: '/' });
  const { data: locations } = useQuery(
    getLocationsQueryOptions(selectedInstance ?? routeData.instances[0].id),
  );

  return (
    <CompactPage
      containerProps={{
        id: 'compact-locations-page',
      }}
    >
      <WindowHeader variant="compact" />
      <div className="locations">
        {(locations ?? routeData.locations).map((location) => (
          <LocationCard
            location={location}
            key={`${location.instance_id}-${location.id}`}
          />
        ))}
      </div>
      <div className="compact-footer">
        <Divider spacing={ThemeSpacing.Md} />
      </div>
    </CompactPage>
  );
};
