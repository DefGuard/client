import './style.scss';
import { type UseQueryResult, useMutation, useQueries } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { Checkbox } from '../../../../../shared/components/Checkbox/Checkbox';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { useAppData } from '../../../../../shared/providers/AppDataContext';
import { Snackbar } from '../../../../../shared/providers/snackbar/snackbar';
import { api } from '../../../../../shared/rust-api/api';
import { getLocationsQueryOptions } from '../../../../../shared/rust-api/query';
import type { LocationInfo } from '../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../shared/types';
import { AutoConnectInstanceCard } from './components/AutoConnectInstanceCard/AutoConnectInstanceCard';
import { buildAutoConnectCards } from './helpers';

type Props = {
  launchAtLogin: boolean;
};

type ToggleArgs = {
  location: LocationInfo;
  autoConnect: boolean;
};

const combineLocations = (results: UseQueryResult<LocationInfo[]>[]) =>
  results.map((result) => result.data ?? []);

export const AutoConnectSection = ({ launchAtLogin }: Props) => {
  const { instances } = useAppData();
  const [eligibleOnly, setEligibleOnly] = useState(true);
  const locationsPerInstance = useQueries({
    queries: instances.map((instance) => getLocationsQueryOptions(instance.id)),
    combine: combineLocations,
  });

  const { mutate: setAutoConnect, isPending } = useMutation({
    mutationFn: ({ location, autoConnect }: ToggleArgs) =>
      api.setLocationAutoConnect({ locationId: location.id, autoConnect }),
    meta: { invalidate: ['locations'] },
    onError: (_error, { location }) => {
      Snackbar.error(`Couldn't change auto-connect for ${location.name}.`);
    },
  });

  const { cards, eligibleCards } = useMemo(
    () => buildAutoConnectCards(instances, locationsPerInstance),
    [instances, locationsPerInstance],
  );
  const showFilter = eligibleCards.length > 0;
  const filterApplied = showFilter && eligibleOnly;
  const visibleCards = filterApplied ? eligibleCards : cards;

  if (cards.length === 0) return null;

  return (
    <div className="auto-connect-section">
      <Divider spacing={ThemeSpacing.Xl} />
      <p className="title">Auto-connect locations</p>
      <SizedBox height={ThemeSpacing.Xs} />
      <p className="description">
        Select the locations to connect automatically when the Defguard client starts.{' '}
        <span>
          Locations with MFA can't auto-connect. Only one All traffic location can
          auto-connect, across all instances.
        </span>
      </p>
      {!launchAtLogin && (
        <>
          <SizedBox height={ThemeSpacing.Xs} />
          <p className="hint">
            Defguard doesn't start with your operating system, so these locations connect
            only when you open the client.
          </p>
        </>
      )}
      <SizedBox height={ThemeSpacing.Xl2} />
      {showFilter && (
        <>
          <Checkbox
            text="Show only locations eligible for auto-connect"
            active={filterApplied}
            onClick={() => setEligibleOnly((value) => !value)}
          />
          <SizedBox height={ThemeSpacing.Lg} />
        </>
      )}
      <div className="instances">
        {visibleCards.map(({ instance, locations }) => (
          <AutoConnectInstanceCard
            key={instance.id}
            name={instance.name}
            locations={locations}
            disabled={isPending}
            onToggle={(location) =>
              setAutoConnect({ location, autoConnect: !location.auto_connect })
            }
          />
        ))}
      </div>
    </div>
  );
};
