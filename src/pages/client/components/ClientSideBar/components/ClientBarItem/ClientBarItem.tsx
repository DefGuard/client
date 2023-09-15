import { autoUpdate, useFloating } from '@floating-ui/react';
import classNames from 'classnames';

import { ActivityIcon } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/types';

export const ClientBarItem = () => {
  const cn = classNames('client-bar-item', 'clickable');

  const active = true;

  const { refs, floatingStyles } = useFloating({
    placement: 'right',
    whileElementsMounted: (refElement, floatingElement, updateFunc) =>
      autoUpdate(refElement, floatingElement, updateFunc),
  });

  return (
    <>
      <div className={cn} ref={refs.setReference}>
        <ActivityIcon
          status={active ? ActivityIconVariant.CONNECTED : ActivityIconVariant.BLANK}
        />
        <p>Placeholder instance name</p>
      </div>
      {active && (
        <div
          className="client-bar-active-item-bar"
          ref={refs.setFloating}
          style={floatingStyles}
        ></div>
      )}
    </>
  );
};
