import { autoUpdate, useFloating } from '@floating-ui/react';
import classNames from 'classnames';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';

import SvgIconConnection from '../../../../../../shared/defguard-ui/components/svg/IconConnection';
import { DefguardInstance } from '../../../../types';

type Props = {
  instance: DefguardInstance;
};

export const ClientBarItem = ({ instance }: Props) => {
  const active = useMemo(() => {
    if (instance.locations.length === 0) return false;
    return !isUndefined(instance.locations.find((l) => l.connected));
  }, [instance.locations]);

  const cn = classNames('client-bar-item', 'clickable', {
    active,
  });

  const { refs, floatingStyles } = useFloating({
    placement: 'right',
    whileElementsMounted: (refElement, floatingElement, updateFunc) =>
      autoUpdate(refElement, floatingElement, updateFunc),
  });

  return (
    <>
      <div className={cn} ref={refs.setReference}>
        <SvgIconConnection className="connection-icon" />
        <p>{instance.name}</p>
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
