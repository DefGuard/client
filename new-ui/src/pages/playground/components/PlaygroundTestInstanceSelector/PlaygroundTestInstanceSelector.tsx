import './style.scss';
import { useState } from 'react';
import { InstanceSelector } from '../../../full/ConfigureMfaPage/components/InstanceSelector/InstanceSelector';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const instances = [
  { id: 1, name: 'Defguard HQ' },
  { id: 2, name: 'Staging' },
  {
    id: 3,
    name: 'Very long instance name that should be truncated or wrapped properly in the selector',
  },
];

export const PlaygroundTestInstanceSelector = () => {
  const [selected, setSelected] = useState<number>();

  return (
    <PlaygroundCard>
      <div className="playground-test-instance-selector">
        <h3>Default</h3>
        <div className="track">
          {instances.map((instance) => (
            <InstanceSelector
              key={instance.id}
              instanceId={instance.id}
              instanceName={instance.name}
              selected={false}
              onClick={() => {}}
            />
          ))}
        </div>
        <h3>Selected</h3>
        <div className="track">
          {instances.map((instance) => (
            <InstanceSelector
              key={instance.id}
              instanceId={instance.id}
              instanceName={instance.name}
              selected
              onClick={() => {}}
            />
          ))}
        </div>
        <h3>Interactive (selected: {selected ?? 'none'})</h3>
        <div className="track">
          {instances.map((instance) => (
            <InstanceSelector
              key={instance.id}
              instanceId={instance.id}
              instanceName={instance.name}
              selected={selected === instance.id}
              onClick={() => setSelected(instance.id)}
            />
          ))}
        </div>
      </div>
    </PlaygroundCard>
  );
};
