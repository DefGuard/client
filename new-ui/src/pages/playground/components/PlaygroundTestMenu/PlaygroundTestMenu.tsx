import './style.scss';
import { useState } from 'react';
import { Menu } from '../../../../shared/components/Menu/Menu';
import { PlaygroundCard } from '../PlaygroundCard/PlaygroundCard';

export const PlaygroundTestMenu = () => {
  const [lastClicked, setLastClicked] = useState<string | null>(null);

  return (
    <PlaygroundCard>
      <div className="playground-test-menu">
        <h3>Menu</h3>
        {lastClicked && <p className="last-clicked">Last clicked: {lastClicked}</p>}
        <Menu
          itemGroups={[
            {
              header: { text: 'Actions' },
              items: [
                { text: 'Edit', icon: 'edit', onClick: () => setLastClicked('Edit') },
                {
                  text: 'Duplicate',
                  icon: 'copy',
                  onClick: () => setLastClicked('Duplicate'),
                },
              ],
            },
            {
              items: [
                {
                  text: 'Delete',
                  icon: 'delete',
                  variant: 'danger',
                  onClick: () => setLastClicked('Delete'),
                },
              ],
            },
          ]}
        />
        <h3>Menu with disabled &amp; nested</h3>
        <Menu
          itemGroups={[
            {
              header: { text: 'File' },
              items: [
                { text: 'New', icon: 'plus', onClick: () => setLastClicked('New') },
                {
                  text: 'Open',
                  icon: 'edit',
                  disabled: true,
                  onClick: () => setLastClicked('Open'),
                },
                {
                  text: 'Export',
                  icon: 'copy',
                  items: [
                    {
                      text: 'Export as PDF',
                      onClick: () => setLastClicked('Export as PDF'),
                    },
                    {
                      text: 'Export as CSV',
                      onClick: () => setLastClicked('Export as CSV'),
                    },
                    {
                      text: 'Export as JSON',
                      disabled: true,
                      onClick: () => setLastClicked('Export as JSON'),
                    },
                  ],
                },
              ],
            },
            {
              header: { text: 'Advanced' },
              items: [
                {
                  text: 'Settings',
                  icon: 'settings',
                  items: [
                    { text: 'General', onClick: () => setLastClicked('General') },
                    { text: 'Security', onClick: () => setLastClicked('Security') },
                  ],
                },
                {
                  text: 'Maintenance',
                  icon: 'protection',
                  disabled: true,
                  onClick: () => setLastClicked('Maintenance'),
                },
              ],
            },
            {
              items: [
                {
                  text: 'Delete',
                  icon: 'delete',
                  variant: 'danger',
                  disabled: true,
                  onClick: () => setLastClicked('Delete (disabled)'),
                },
              ],
            },
          ]}
        />
      </div>
    </PlaygroundCard>
  );
};
