import './style.scss';
import { useState } from 'react';
import { IconKind } from '../../../../shared/components/Icon';
import { SelectionItem } from '../../../../shared/components/SelectionItem/SelectionItem';
import { SelectionItemVariant } from '../../../../shared/components/SelectionItem/types';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';
import { PlaygroundGroup } from '../PlaygroundGroup/PlaygroundGroup';

const radioItems = ['defguard.example.com', 'vpn.example.org'];
const checkboxItems = ['Office network', 'Staging cluster'];

export const PlaygroundTestSelectionItem = () => {
  const [radioValue, setRadioValue] = useState(radioItems[0]);
  const [checked, setChecked] = useState<string[]>([checkboxItems[0]]);

  const toggleChecked = (item: string) => {
    setChecked((current) =>
      current.includes(item)
        ? current.filter((picked) => picked !== item)
        : [...current, item],
    );
  };

  return (
    <PlaygroundCard>
      <div className="playground-test-selection-item">
        <h3>SelectionItem</h3>
        <PlaygroundGroup title="Radio variant">
          {radioItems.map((item) => (
            <SelectionItem
              key={item}
              text={item}
              icon={IconKind.Globe}
              selected={radioValue === item}
              onClick={() => {
                setRadioValue(item);
              }}
            />
          ))}
          <SelectionItem
            text="legacy.example.net"
            icon={IconKind.Globe}
            badge="Unsupported"
            selected={false}
            disabled
          />
        </PlaygroundGroup>
        <PlaygroundGroup title="Checkbox variant">
          {checkboxItems.map((item) => (
            <SelectionItem
              key={item}
              text={item}
              icon={IconKind.Globe}
              variant={SelectionItemVariant.Checkbox}
              selected={checked.includes(item)}
              onClick={() => {
                toggleChecked(item);
              }}
            />
          ))}
          <SelectionItem
            text="Production cluster"
            icon={IconKind.Globe}
            variant={SelectionItemVariant.Checkbox}
            badge="Locked"
            selected
            disabled
          />
        </PlaygroundGroup>
      </div>
    </PlaygroundCard>
  );
};
