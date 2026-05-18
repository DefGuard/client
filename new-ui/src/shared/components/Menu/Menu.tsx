import { Fragment } from 'react';
import { MenuItem } from './components/MenuItem';
import './style.scss';
import clsx from 'clsx';
import { isPresent } from '../../utils/isPresent';
import { MenuHeader } from './components/MenuHeader';
import { MenuSpacer } from './components/MenuSpacer';
import type { MenuProps } from './types';

export const Menu = ({
  itemGroups,
  ref,
  className,
  onClose,
  testId,
  ...props
}: MenuProps) => {
  return (
    <div className={clsx('menu', className)} ref={ref} data-testid={testId} {...props}>
      {itemGroups.map((group, groupIndex) => (
        <Fragment key={group.header?.text ?? groupIndex}>
          {isPresent(group.header) && <MenuHeader {...group.header} onClose={onClose} />}
          {group.items.map((item) => (
            <MenuItem key={item.text} {...item} onClose={onClose} />
          ))}
          {groupIndex !== itemGroups.length - 1 && itemGroups.length !== 1 && (
            <MenuSpacer />
          )}
        </Fragment>
      ))}
    </div>
  );
};
