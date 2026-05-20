import { useState } from 'react';
import { Select } from '../../shared/components/Select/Select';
import type {
  SelectOption,
  SelectOptionGroup,
} from '../../shared/components/Select/types';

type RegionOption = {
  code: string;
};

const quickOptions: readonly SelectOption<RegionOption>[] = [
  {
    key: 'all',
    label: 'All Regions',
    value: { code: 'all' },
  },
];

const groupedOptions: readonly SelectOptionGroup<RegionOption>[] = [
  {
    key: 'eu',
    label: 'Europe',
    options: [
      {
        key: 'de',
        label: 'Germany',
        value: { code: 'de' },
      },
      {
        key: 'fr',
        label: 'France',
        value: { code: 'fr' },
      },
    ],
  },
  {
    key: 'americas',
    label: 'Americas',
    options: [
      {
        key: 'us',
        label: 'United States',
        value: { code: 'us' },
      },
      {
        key: 'ca',
        label: 'Canada',
        value: { code: 'ca' },
      },
    ],
  },
];

export const PlaygroundIndex = () => {
  const [value, setValue] = useState<SelectOption<RegionOption>>(quickOptions[0]);

  return (
    <main id="playground-index">
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--spacing-lg)',
          alignContent: 'center',
          justifyContent: 'flex-start',
          alignItems: 'center',
          width: '100%',
          maxWidth: 560,
          margin: '0 auto',
        }}
      >
        <Select
          label="Destination"
          placeholder="Select destination"
          groups={groupedOptions}
          value={value}
          onChange={setValue}
        />
      </div>
    </main>
  );
};
