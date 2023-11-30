import { autoUpdate, useFloating } from '@floating-ui/react';
import classNames from 'classnames';
import { useMatch, useNavigate } from 'react-router-dom';

import SvgIconConnection from '../../../../../../shared/defguard-ui/components/svg/IconConnection';
import { routes } from '../../../../../../shared/routes';
import { useClientStore } from '../../../../hooks/useClientStore';
import { DefguardInstance } from '../../../../types';

type Props = {
  instance: DefguardInstance;
};

export const ClientBarItem = ({ instance }: Props) => {
  const instancePage = useMatch('/client/');
  const navigate = useNavigate();
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
        onClick={() => {
          setClientStore({ selectedInstance: instance.id });
          if (!instancePage) {
            navigate(routes.client.base, { replace: true });
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
