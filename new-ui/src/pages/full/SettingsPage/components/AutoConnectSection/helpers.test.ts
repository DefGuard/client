import { describe, expect, it } from 'vitest';
import {
  AutoConnectBlock,
  type AutoConnectBlockValue,
  type InstanceInfo,
  type LocationInfo,
} from '../../../../../shared/rust-api/types';
import { buildAutoConnectCards } from './helpers';

const instanceOf = (id: number) => ({ id, name: `instance-${id}` }) as InstanceInfo;

const locationOf = (
  id: number,
  auto_connect_blocked: AutoConnectBlockValue | null = null,
) => ({ id, auto_connect_blocked }) as LocationInfo;

const idsOf = (cards: { instance: InstanceInfo; locations: LocationInfo[] }[]) =>
  cards.map(({ instance, locations }) => [instance.id, locations.map(({ id }) => id)]);

describe('buildAutoConnectCards', () => {
  it('drops instances without locations, including ones still loading', () => {
    const { cards } = buildAutoConnectCards(
      [instanceOf(1), instanceOf(2), instanceOf(3)],
      [[locationOf(10)], []],
    );
    expect(idsOf(cards)).toEqual([[1, [10]]]);
  });

  it('keeps locations blocked by all traffic as eligible', () => {
    const { eligibleCards } = buildAutoConnectCards(
      [instanceOf(1)],
      [[locationOf(10), locationOf(11, AutoConnectBlock.AllTrafficTaken)]],
    );
    expect(idsOf(eligibleCards)).toEqual([[1, [10, 11]]]);
  });

  it('leaves MFA locations out of the eligible cards', () => {
    const { cards, eligibleCards } = buildAutoConnectCards(
      [instanceOf(1), instanceOf(2)],
      [
        [locationOf(10), locationOf(11, AutoConnectBlock.Mfa)],
        [locationOf(20, AutoConnectBlock.Mfa)],
      ],
    );
    expect(idsOf(cards)).toEqual([
      [1, [10, 11]],
      [2, [20]],
    ]);
    expect(idsOf(eligibleCards)).toEqual([[1, [10]]]);
  });
});
