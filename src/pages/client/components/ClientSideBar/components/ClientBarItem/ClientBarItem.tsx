import { autoUpdate, useFloating } from '@floating-ui/react';
import classNames from 'classnames';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { useMatch, useNavigate } from 'react-router-dom';

import SvgIconConnection from '../../../../../../shared/defguard-ui/components/svg/IconConnection';
import { routes } from '../../../../../../shared/routes';
import { useClientStore } from '../../../../hooks/useClientStore';
import { WireguardInstanceType } from '../../../../types';

type Props = {
  itemType: WireguardInstanceType;
  itemId: number;
  label: string;
  active?: boolean;
};

export const ClientBarItem = ({ itemType, itemId, label, active = false }: Props) => {
  const instancePage = useMatch('/client/instance/');
  const navigate = useNavigate();
  const setClientStore = useClientStore((state) => state.setState);
  const selectedInstance = useClientStore((state) => state.selectedInstance);
  const itemSelected = useMemo(() => {
    return (
      !isUndefined(selectedInstance) &&
      !isUndefined(selectedInstance?.id) &&
      selectedInstance.id === itemId &&
      selectedInstance.type === itemType
    );
  }, [selectedInstance, itemType, itemId]);

  const cn = classNames('client-bar-item', 'clickable', {
    active: itemSelected,
    connected: active,
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
          setClientStore({
            selectedInstance: {
              id: itemId,
              type: itemType,
            },
          });
          if (!instancePage) {
            navigate(routes.client.instancePage, { replace: true });
          }
        }}
      >
        <SvgIconConnection className="connection-icon" />
        <p>{label}</p>
        <div className="instance-shorted">
          <SvgIconConnection className="connection-icon" />
          <p>{label[0]}</p>
        </div>
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
