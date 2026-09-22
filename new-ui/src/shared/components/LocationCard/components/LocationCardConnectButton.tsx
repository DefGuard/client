import { useMutation } from '@tanstack/react-query';
import { error as logError } from '@tauri-apps/plugin-log';
import { Snackbar } from '../../../providers/snackbar/snackbar';
import { api } from '../../../rust-api/api';
import { connectConfigureFactorsSource } from '../../../utils/configureFactorsSource';
import { ConnectionAbility, shouldStartMfa } from '../../../utils/mfa';
import { IconKind } from '../../Icon';
import { parseConnectError } from '../api/connectError';
import { useLocationCardContext } from '../context/context';
import { LocationCardViews } from '../context/types';
import { ConnectButton } from './ConnectButton/ConnectButton';

export const LocationCardConnectButton = () => {
  const { location, connectionAbility, setPostureError, setView, startMfa } =
    useLocationCardContext();

  const canConfigureMfa = connectionAbility === ConnectionAbility.Configurable;

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
      } else if (connectError?.kind === 'allTrafficConflict') {
        setView(LocationCardViews.ConnectionError, connectError.message);
      } else if (connectError?.kind === 'serviceUnavailable') {
        setView(LocationCardViews.ConnectionError);
      }
    },
    meta: {
      invalidate: ['locations'],
    },
  });

  const { mutate: configureMfa, isPending: isOpeningConfiguration } = useMutation({
    mutationFn: api.initiateConfigureFactorScreen,
    onError: (err) => {
      void logError(`Failed to open the MFA configuration screen: ${err}`);
      Snackbar.error('Could not open MFA configuration.');
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

  const isBusy = isConnecting || isDisconnecting || isOpeningConfiguration;

  const handleClick = () => {
    if (location.active) {
      disconnect({
        connectionType: location.connection_type,
        locationId: location.id,
      });
    } else if (canConfigureMfa) {
      // Handing over to the backend, so the wizard opens in the full view from either window.
      configureMfa({
        instanceId: location.instance_id,
        methods: [],
        source: connectConfigureFactorsSource(),
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
    <ConnectButton
      icon={canConfigureMfa ? IconKind.ManageKeys : null}
      text={canConfigureMfa ? 'Configure MFA' : null}
      active={location.active}
      onClick={handleClick}
      disabled={
        isBusy ||
        (!location.active && connectionAbility === ConnectionAbility.Unavailable)
      }
    />
  );
};
