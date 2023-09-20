import { autoUpdate, useFloating } from '@floating-ui/react';
import classNames from 'classnames';

import SvgIconConnection from '../../../../../../shared/defguard-ui/components/svg/IconConnection';
import { useClientStore } from '../../../../hooks/useClientStore';
import { DefguardInstance } from '../../../../types';

type Props = {
  instance: DefguardInstance;
};

export const ClientBarItem = ({ instance }: Props) => {
  const setClientStore = useClientStore((state) => state.setState);
  const selectedInstance = useClientStore((state) => state.selectedInstance);
  const cn = classNames('client-bar-item', 'clickable', {
    active: instance.id === selectedInstance,
    connected: instance.connected,
  });

  const { refs, floatingStyles } = useFloating({
    placement: 'right',
    whileElementsMounted: (refElement, floatingElement, updateFunc) =>
      autoUpdate(refElement, floatingElement, updateFunc),
  });

  return (
    <>
      <div
        className={cn}
        ref={refs.setReference}
        onClick={() => setClientStore({ selectedInstance: instance.id })}
      >
        <SvgIconConnection className="connection-icon" />
        <p>{instance.name}</p>
      </div>
      {instance.connected && (
        <div
          className="client-bar-active-item-bar"
          ref={refs.setFloating}
          style={floatingStyles}
        ></div>
      )}
    </>
  );
};
