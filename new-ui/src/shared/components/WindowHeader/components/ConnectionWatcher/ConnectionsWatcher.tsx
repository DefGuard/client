import './style.scss';
import {
  autoUpdate,
  FloatingPortal,
  size as floatingSize,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import clsx from 'clsx';
import { useEffect, useState } from 'react';
import { api } from '../../../../rust-api/api';
import { ThemeSpacing, ThemeVariable } from '../../../../types';
import { isPresent } from '../../../../utils/isPresent';
import { Divider } from '../../../Divider/Divider';
import { FloatingMenu } from '../../../FloatingMenu/FloatingMenu';
import { Icon } from '../../../Icon';

export const ConnectionWatcher = () => {
  const { mutate: disconnect } = useMutation({
    mutationFn: api.disconnectLocations,
  });

  const { data: connections } = useQuery({
    queryKey: ['alive-connection'],
    queryFn: api.getAllActiveConnections,
    refetchInterval: 5_000,
  });

  const connected = (connections?.length ?? 0) > 0;

  const [floatingOpen, setFloatingOpen] = useState(false);

  useEffect(() => {
    if (!connected) {
      setFloatingOpen(false);
    }
  }, [connected]);

  const { refs, context, floatingStyles } = useFloating({
    placement: 'bottom-start',
    open: floatingOpen,
    onOpenChange: setFloatingOpen,
    middleware: [
      offset(4),
      shift(),
      floatingSize({
        apply({ rects, elements }) {
          elements.floating.style.minWidth = `${rects.reference.width}px`;
        },
      }),
    ],
    whileElementsMounted: autoUpdate,
  });

  const click = useClick(context, {
    toggle: true,
    enabled: connected,
  });

  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    outsidePress: true,
  });

  const { getFloatingProps, getReferenceProps } = useInteractions([click, dismiss]);

  return (
    <>
      <div
        className={clsx('connection-watcher', {
          connected,
        })}
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        {!connected && <p className="no-connection-label">Not connected</p>}
        {connected && isPresent(connections) && (
          <div className="connected-row">
            <Icon size={16} staticColor={ThemeVariable.FgAction} icon="online" />
            <p>{`Connected (${connections.length})`}</p>
            <Icon
              size={16}
              icon="arrow-small"
              staticColor="var(--fg-action)"
              rotationDirection={floatingOpen ? 'down' : 'right'}
            />
          </div>
        )}
      </div>
      {floatingOpen && (
        <FloatingPortal>
          <FloatingMenu
            containerProps={{
              ref: refs.setFloating,
              style: { position: 'absolute', ...floatingStyles },
              ...getFloatingProps(),
              className: 'connection-watcher-floating',
            }}
          >
            <p className="label">Connected locations</p>
            {connections?.map((con) => (
              <div className="connection" key={con.id}>
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 16 16"
                  fill="none"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <circle cx="8" cy="8" r="4" fill="#74FFB8" />
                </svg>
                <p>{con.name}</p>
              </div>
            ))}
            <Divider spacing={ThemeSpacing.Sm} />
            <button
              className="disconnect"
              onClick={() => {
                const ids = connections?.map((c) => c.id);
                if (ids) {
                  disconnect(ids);
                }
              }}
            >
              <Icon size={16} icon="disconnect-all" />
              <p>Disconnect all</p>
            </button>
          </FloatingMenu>
        </FloatingPortal>
      )}
    </>
  );
};
