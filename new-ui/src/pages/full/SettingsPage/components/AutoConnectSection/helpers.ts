import {
  AutoConnectBlock,
  type InstanceInfo,
  type LocationInfo,
} from '../../../../../shared/rust-api/types';

export type AutoConnectCard = {
  instance: InstanceInfo;
  locations: LocationInfo[];
};

const isEligible = (location: LocationInfo) =>
  location.auto_connect_blocked !== AutoConnectBlock.Mfa &&
  location.auto_connect_blocked !== AutoConnectBlock.ServiceLocation;

const withLocations = (cards: AutoConnectCard[]) =>
  cards.filter(({ locations }) => locations.length > 0);

export const buildAutoConnectCards = (
  instances: InstanceInfo[],
  locationsPerInstance: LocationInfo[][],
) => {
  const cards = withLocations(
    instances.map((instance, index) => ({
      instance,
      locations: locationsPerInstance[index] ?? [],
    })),
  );
  const eligibleCards = withLocations(
    cards.map((card) => ({ ...card, locations: card.locations.filter(isEligible) })),
  );
  return { cards, eligibleCards };
};
