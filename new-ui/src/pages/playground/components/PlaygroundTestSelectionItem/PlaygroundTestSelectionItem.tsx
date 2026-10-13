import './style.scss';
import { useState } from 'react';
import { IconKind } from '../../../../shared/components/Icon';
import { SelectionItem } from '../../../../shared/components/SelectionItem/SelectionItem';
import { SelectionItemVariant } from '../../../../shared/components/SelectionItem/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

const items = [
  { id: 1, text: 'Authenticator app', icon: IconKind.MobileLock },
  { id: 2, text: 'Security key', icon: IconKind.Key },
  { id: 3, text: 'Network settings', icon: IconKind.NetworkSettings },
];

export const PlaygroundTestSelectionItem = () => {
  const [radio, setRadio] = useState<number>();
  const [checked, setChecked] = useState<number[]>([]);

  return (
    <PlaygroundCard>
      <div className="playground-test-selection-item">
        <h3>Radio</h3>
        <div className="track">
          {items.map((item) => (
            <SelectionItem
              key={item.id}
              text={item.text}
              icon={item.icon}
              selected={radio === item.id}
              onClick={() => setRadio(item.id)}
            />
          ))}
        </div>
        <h3>Checkbox</h3>
        <div className="track">
          {items.map((item) => (
            <SelectionItem
              key={item.id}
              text={item.text}
              icon={item.icon}
              variant={SelectionItemVariant.Checkbox}
              selected={checked.includes(item.id)}
              onClick={() =>
                setChecked((current) =>
                  current.includes(item.id)
                    ? current.filter((id) => id !== item.id)
                    : [...current, item.id],
                )
              }
            />
          ))}
        </div>
        <h3>Badge</h3>
        <div className="track">
          <SelectionItem
            text="Label"
            icon={IconKind.Globe}
            badge="Text"
            selected={radio === 4}
            onClick={() => setRadio(4)}
          />
          <SelectionItem
            text="A very long label that should be truncated before the badge"
            icon={IconKind.Globe}
            badge="Text"
            selected={radio === 5}
            onClick={() => setRadio(5)}
          />
          <SelectionItem
            text="Disabled"
            icon={IconKind.Globe}
            badge="Text"
            selected={false}
            disabled
          />
        </div>
        <h3>Disabled</h3>
        <div className="track">
          <SelectionItem text="Radio" icon={IconKind.Globe} selected={false} disabled />
          <SelectionItem text="Radio selected" icon={IconKind.Globe} selected disabled />
          <SelectionItem
            text="Checkbox"
            icon={IconKind.Globe}
            variant={SelectionItemVariant.Checkbox}
            selected={false}
            disabled
          />
          <SelectionItem
            text="Checkbox selected"
            icon={IconKind.Globe}
            variant={SelectionItemVariant.Checkbox}
            selected
            disabled
          />
        </div>
      </div>
    </PlaygroundCard>
  );
};
