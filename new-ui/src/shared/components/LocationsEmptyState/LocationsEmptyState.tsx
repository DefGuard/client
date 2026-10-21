import './style.scss';
import { EmptyState } from '../EmptyState/EmptyState';
import { EmptyIcon } from '../EmptyState/types';

export const LocationsEmptyState = () => {
  return (
    <EmptyState
      className="locations-empty-state"
      icon={EmptyIcon.Locations}
      title="No locations"
      subtitle="You don't have any locations added yet. Contact your administrator to get access."
    />
  );
};
