import { useMutation } from '@tanstack/react-query';
import { api } from '../../../rust-api/api';
import { shouldStartMfa } from '../../../utils/mfa';
import { parseConnectError } from '../api/connectError';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';
import { ConnectButton } from './ConnectButton/ConnectButton';

export const LocationCardConnectButton = () => {
  const { location, setPostureError, setView, startMfa } = useLocationCardContext();

  const { mutate: connect, isPending: isConnecting } = useMutation({
    mutationFn: api.connect,
    onSuccess: () => {
      setView(LocationCardViews.Connected);
    },
    onError: (err) => {
      const connectError = parseConnectError(err);

      if (
        location.posture_check_required &&
        connectError?.kind === 'postureCheckFailed'
      ) {
        setPostureError(connectError.message);
        setView(LocationCardViews.PostureCheckFail);
      } else if (connectError?.kind === 'serviceUnavailable') {
        setView(LocationCardViews.ConnectionError);
      }
    },
    meta: {
      invalidate: ['locations'],
    },
  });

  const { mutate: disconnect, isPending: isDisconnecting } = useMutation({
    mutationFn: api.disconnect,
    onSuccess: () => {
      setView(LocationCardViews.Default);
    },
    meta: {
      invalidate: ['locations'],
    },
  });

  const isBusy = isConnecting || isDisconnecting;

  const handleClick = () => {
    if (location.active) {
      disconnect({
        connectionType: location.connection_type,
        locationId: location.id,
      });
    } else if (shouldStartMfa(location)) {
      startMfa();
    } else {
      connect({
        connectionType: location.connection_type,
        locationId: location.id,
      });
    }
  };

  return (
    <ConnectButton active={location.active} onClick={handleClick} disabled={isBusy} />
  );
};
