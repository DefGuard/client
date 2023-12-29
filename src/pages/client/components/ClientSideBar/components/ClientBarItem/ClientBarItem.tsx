import { autoUpdate, useFloating } from '@floating-ui/react';
import classNames from 'classnames';
import { useMatch, useNavigate } from 'react-router-dom';

import SvgIconConnection from '../../../../../../shared/defguard-ui/components/svg/IconConnection';
import { routes } from '../../../../../../shared/routes';
import { useClientStore } from '../../../../hooks/useClientStore';
import { WireguardInstanceType } from '../../../../types';

// Define a generic interface for the type with required fields
interface BaseInstance {
  id?: number;
  name: string;
  connected: boolean;
  type: WireguardInstanceType;
}

// Extend the generic type in the Props interface
type Props<T extends BaseInstance> = {
  instance: T;
};

export const ClientBarItem = <T extends BaseInstance>({ instance }: Props<T>) => {
  const instancePage = useMatch('/client/');
  const navigate = useNavigate();
  const setClientStore = useClientStore((state) => state.setState);
  const selectedInstance = useClientStore((state) => state.selectedInstance);

  // FIXME: Fix tunnel active when detail will be implemented
  const active =
    instance.type === WireguardInstanceType.TUNNEL
      ? routes.client.tunnelPage + instance.id === window.location.pathname
      : instance.type === WireguardInstanceType.DEFGUARD_INSTANCE
        ? instance.id === selectedInstance?.id
        : false;

  const cn = classNames('client-bar-item', 'clickable', {
    active: active,
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
        onClick={() => {
          if (instance.type === WireguardInstanceType.DEFGUARD_INSTANCE) {
            setClientStore({
              selectedInstance: {
                id: instance.id as number,
                type: WireguardInstanceType.DEFGUARD_INSTANCE,
              },
            });
            if (!instancePage) {
              navigate(routes.client.base, { replace: true });
            }
          } else {
            setClientStore({
              selectedInstance: {
                id: instance.id as number,
                type: WireguardInstanceType.TUNNEL,
              },
            });
            navigate(routes.client.tunnelPage);
          }
        }}
      >
        <SvgIconConnection className="connection-icon" />
        <p>{instance.name}</p>
        <div className="instance-shorted">
          <SvgIconConnection className="connection-icon" />
          <p>{instance.name[0]}</p>
        </div>
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
