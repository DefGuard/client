import { createWithEqualityFn } from 'zustand/traditional';

import { DefguardInstance } from '../types';

const mockValues: StoreValues = {
  instances: [
    {
      id: 'instance-1',
      name: 'Teonite',
      locations: [
        {
          id: 'location-teonite-1',
          ip: '169.254.0.0',
          name: 'Szczecin',
          connected: false,
        },
        {
          id: 'location-teonite-2',
          ip: '169.253.0.0',
          name: 'Zurich',
          connected: false,
        },
        {
          id: 'location-teonite-3',
          ip: '169.252.0.0',
          name: 'Monaco',
          connected: true,
        },
        {
          id: 'location-teonite-4',
          ip: '169.251.0.0',
          name: 'Berlin',
          connected: false,
        },
        {
          id: 'location-teonite-5',
          ip: '169.250.0.0',
          name: 'Paris',
          connected: false,
        },
        {
          id: 'location-teonite-6',
          ip: '169.249.0.0',
          name: 'US East',
          connected: true,
        },
      ],
    },
    {
      id: 'instance-2',
      name: 'WideStreet',
      locations: [
        {
          id: 'location-widestreet-1',
          ip: '169.254.0.0',
          name: 'Szczecin',
          connected: false,
        },
        {
          id: 'location-widestreet-2',
          ip: '169.253.0.0',
          name: 'Zurich',
          connected: false,
        },
        {
          id: 'location-widestreet-3',
          ip: '169.252.0.0',
          name: 'Monaco',
          connected: false,
        },
        {
          id: 'location-widestreet-4',
          ip: '169.251.0.0',
          name: 'Berlin',
          connected: false,
        },
        {
          id: 'location-widestreet-5',
          ip: '169.250.0.0',
          name: 'Paris',
          connected: false,
        },
        {
          id: 'location-widestreet-6',
          ip: '169.249.0.0',
          name: 'US East',
          connected: false,
        },
      ],
    },
  ],
  selectedInstance: 'instance-1',
};

// eslint-disable-next-line
const defaultValues: StoreValues = {
  instances: [],
  selectedInstance: undefined,
};

export const useClientStore = createWithEqualityFn<Store>(
  (set) => ({ ...mockValues, setState: (values) => set({ ...values }) }),
  Object.is,
);

type Store = StoreValues & StoreMethods;

type StoreValues = {
  instances: DefguardInstance[];
  selectedInstance?: DefguardInstance['id'];
};

type StoreMethods = {
  setState: (values: Partial<StoreValues>) => void;
};
