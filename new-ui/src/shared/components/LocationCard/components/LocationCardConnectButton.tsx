import { useMutation } from '@tanstack/react-query';
import { api } from '../../../rust-api/api';
import { LocationMfaMode } from '../../../rust-api/types';
import { parseConnectError } from '../api/connectError';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';
import { ConnectButton } from './ConnectButton/ConnectButton';

export const LocationCardConnectButton = () => {
  const { location, setPostureError, setView, startMfa } = useLocationCardContext();

  const { mutate: connect } = useMutation({
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
      }
    },
    meta: {
      invalidate: ['locations'],
    },
  });

  const { mutate: disconnect } = useMutation({
    mutationFn: api.disconnect,
    onSuccess: () => {
      setView(LocationCardViews.Default);
    },
    meta: {
      invalidate: ['locations'],
    },
  });

  const handleClick = () => {
    if (location.active) {
      disconnect({
        connectionType: location.connection_type,
        locationId: location.id,
      });
    } else if (location.location_mfa_mode !== LocationMfaMode.Disabled) {
      startMfa();
    } else {
      connect({
        connectionType: location.connection_type,
        locationId: location.id,
      });
    }
  };

  return <ConnectButton active={location.active} onClick={handleClick} />;
};
