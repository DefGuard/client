import { fromUint8Array } from 'js-base64';
import { useEffect, useMemo } from 'react';
import QrCode from 'react-qr-code';
import useWebSocket from 'react-use-websocket';
import z from 'zod';
import { shallow } from 'zustand/shallow';
import { useToaster } from '../../../../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../../../../../clientAPI/clientApi';
import type { CommonWireguardFields } from '../../../../../../../../types';
import { useMFAModal } from '../../useMFAModal';
import './style.scss';
import { Button } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { MessageBox } from '../../../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';

type MfaMobileQrData = {
  token: string;
  challenge: string;
};

type Props = {
  data: MfaMobileQrData;
  proxyUrl: string;
  instanceUuid: string;
  onCancel: () => void;
};

const { connect } = clientApi;

const wsResponseSchema = z.object({
  type: z.string().min(1),
  preshared_key: z.string().min(1),
});

export const MfaMobileApprove = ({
  data: { challenge, token },
  proxyUrl,
  instanceUuid,
  onCancel,
}: Props) => {
  const toaster = useToaster();
  const [closeModal] = useMFAModal((s) => [s.close], shallow);
  const location = useMFAModal((s) => s.instance as CommonWireguardFields);

  const wsUrl = useMemo(
    () =>
      `${proxyUrl.replace('http', 'ws').replace('https', 'wss')}api/v1/client-mfa/remote`,
    [proxyUrl],
  );

  const { lastMessage, readyState } = useWebSocket(wsUrl, {
    queryParams: {
      token,
    },
    onOpen: () => {
      console.log('Websocket connected');
    },
    onClose: () => {
      console.log('Websocket closed');
    },
  });

  const qrString = useMemo(() => {
    const data = {
      token,
      challenge,
      instance_id: instanceUuid,
    };
    const jsonString = JSON.stringify(data);
    const textEncoder = new TextEncoder();
    const encoded = textEncoder.encode(jsonString);
    return fromUint8Array(encoded);
  }, [token, challenge, instanceUuid]);

  useEffect(() => {
    console.log(`Last msg: ${lastMessage}\nState ${readyState}`);
  }, [lastMessage, readyState]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: Side effect
  useEffect(() => {
    if (lastMessage != null) {
      const parsed = JSON.parse(lastMessage.data);
      const schemaResult = wsResponseSchema.safeParse(parsed);
      if (schemaResult.success) {
        connect({
          connectionType: location.connection_type,
          locationId: location.id,
          presharedKey: schemaResult.data.preshared_key,
        })
          .then(() => {
            closeModal();
            toaster.success('Connection authorized.');
          })
          .catch((e) => {
            console.error(e);
          });
      } else {
        // catch possible changes in api
        toaster.error('Unknown response from proxy');
      }
    }
  }, [lastMessage]);

  return (
    <div id="mobile-approve-mfa">
      <MessageBox message="Scan this QR with the defguard mobile app from the same instance screen." />
      <QrCode value={qrString} />
      <Button text="Cancel" onClick={onCancel} />
    </div>
  );
};
