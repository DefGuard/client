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
import { debug, error } from '@tauri-apps/plugin-log';
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

  var manuallyCancelled = false;
  const { getWebSocket, lastMessage } = useWebSocket(wsUrl, {
    queryParams: {
      token,
    },
    onClose: () => {
      debug('WebSocket connection to proxy for mobile app MFA closed.');
    },
    onError: () => {
      if (!manuallyCancelled) {
        toaster.error('Unexpected error in WebSocket connection to proxy');
        error(
          'MFA auth using mobile app failed. Unexpected error in WebSocket connection to proxy.',
        );
        // go back to previous step
        onCancel();
      }
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

  const cancel = () => {
    manuallyCancelled = true;
    const socket = getWebSocket();
    socket?.close();
    // go back to previous step
    onCancel();
  }

  return (
    <div id="mobile-approve-mfa">
      <MessageBox message="Scan this QR with Defguard mobile app from the same instance screen.">
        <p>
          <span>
            {'Go to the mobile app, select this instance and click the Biometry button'}
          </span>
          <ExampleButton />
          <span>{'in the bottom right corner.'}</span>
        </p>
      </MessageBox>
      <QrCode value={qrString} />
      <Button text="Cancel" onClick={cancel} />
    </div>
  );
};

const ExampleButton = () => {
  return (
    <div className="example-mobile-button">
      <QrIcon />
    </div>
  );
};

const QrIcon = () => {
  return (
    <svg
      width="22"
      height="22"
      viewBox="0 0 22 22"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M1.02151 8.81162L0.917016 8.80673C0.401354 8.75382 0 8.31229 0 7.7745V1.03712C1.95739e-05 0.499341 0.401362 0.0577882 0.917016 0.00488286L1.02151 0H7.65449C8.21928 0 8.67598 0.463589 8.676 1.03712V7.7745C8.676 8.34805 8.21929 8.81162 7.65449 8.81162H1.02151ZM2.00005 6.81159H6.67595V2.00002H2.00005V6.81159Z"
        fill="white"
      />
      <path
        d="M14.3455 8.81162L14.241 8.80673C13.7253 8.75382 13.324 8.31229 13.324 7.7745V1.03712C13.324 0.499341 13.7253 0.0577882 14.241 0.00488286L14.3455 0H20.9785C21.5433 0 22 0.463589 22 1.03712V7.7745C22 8.34805 21.5433 8.81162 20.9785 8.81162H14.3455ZM15.324 6.81159H19.9999V2.00002H15.324V6.81159Z"
        fill="white"
      />
      <path
        d="M1.02151 22L0.917016 21.9951C0.401354 21.9422 0 21.5006 0 20.9629V14.2255C1.95739e-05 13.6877 0.401362 13.2461 0.917016 13.1932L1.02151 13.1884H7.65449C8.21928 13.1884 8.67598 13.6519 8.676 14.2255V20.9629C8.676 21.5364 8.21929 22 7.65449 22H1.02151ZM2.00005 19.9999H6.67595V15.1884H2.00005V19.9999Z"
        fill="white"
      />
      <path
        d="M18.885 17.3795H18.1823C17.8953 17.3795 17.6625 17.143 17.6625 16.8516V16.138C17.6625 15.8466 17.8953 15.6101 18.1823 15.6101H18.885C19.172 15.6101 19.4048 15.8466 19.4048 16.138V16.8516C19.4048 17.143 19.172 17.3795 18.885 17.3795Z"
        fill="white"
      />
      <path
        d="M21.0777 19.5782H20.374C20.0871 19.5782 19.8552 19.3417 19.8552 19.0503V18.3367C19.8552 18.0453 20.0871 17.8088 20.374 17.8088H21.0777C21.3637 17.8088 21.5966 18.0453 21.5966 18.3367V19.0503C21.5966 19.3417 21.3637 19.5782 21.0777 19.5782Z"
        fill="white"
      />
      <path
        d="M18.885 21.776H18.1823C17.8953 21.776 17.6625 21.5395 17.6625 21.2481V20.5345C17.6625 20.2431 17.8953 20.0066 18.1823 20.0066H18.885C19.172 20.0066 19.4048 20.2431 19.4048 20.5345V21.2481C19.4048 21.5395 19.172 21.776 18.885 21.776Z"
        fill="white"
      />
      <path
        d="M4.68991 5.29049H3.98725C3.70027 5.29049 3.46741 5.05402 3.46741 4.76259V4.04903C3.46741 3.75759 3.70027 3.52112 3.98725 3.52112H4.68991C4.9769 3.52112 5.20976 3.75759 5.20976 4.04903V4.76259C5.20976 5.05402 4.9769 5.29049 4.68991 5.29049Z"
        fill="white"
      />
      <path
        d="M4.68991 18.6925H3.98725C3.70027 18.6925 3.46741 18.456 3.46741 18.1646V17.451C3.46741 17.1596 3.70027 16.9231 3.98725 16.9231H4.68991C4.9769 16.9231 5.20976 17.1596 5.20976 17.451V18.1646C5.20976 18.456 4.9769 18.6925 4.68991 18.6925Z"
        fill="white"
      />
      <path
        d="M16.6361 17.8079H16.1581H15.9334H13.7407C13.4537 17.8079 13.2208 18.0443 13.2208 18.3358V19.0493C13.2208 19.3408 13.4537 19.5772 13.7407 19.5772H15.9334H16.1581H16.6361C16.9231 19.5772 17.1559 19.3408 17.1559 19.0493V18.3358C17.1559 18.0443 16.9231 17.8079 16.6361 17.8079Z"
        fill="white"
      />
      <path
        d="M21.0777 11.0996H20.4588H20.3751H18.1823C17.8953 11.0996 17.6625 11.3361 17.6625 11.6275V12.3411C17.6625 12.6325 17.8953 12.869 18.1823 12.869H19.8552V14.6539C19.8552 14.9454 20.0881 15.1818 20.3751 15.1818H21.0777C21.3647 15.1818 21.5976 14.9454 21.5976 14.6539V12.3411V12.1274V11.6275C21.5976 11.3361 21.3647 11.0996 21.0777 11.0996Z"
        fill="white"
      />
      <path
        d="M16.6362 13.4124H14.9633V11.6285C14.9633 11.3371 14.7304 11.1006 14.4434 11.1006H14.0502H13.7408H11.5766C11.2896 11.1006 11.0568 11.3371 11.0568 11.6285V12.3421C11.0568 12.6335 11.2896 12.87 11.5766 12.87H13.2209V13.9403V14.155V14.6539V16.8526C13.2209 17.1441 13.4538 17.3805 13.7408 17.3805H14.4434C14.7304 17.3805 14.9633 17.1441 14.9633 16.8526V15.1818H16.6362C16.9232 15.1818 17.156 14.9453 17.156 14.6539V13.9403C17.156 13.6489 16.9232 13.4124 16.6362 13.4124Z"
        fill="white"
      />
      <path
        d="M12.2793 21.776H11.5766C11.2896 21.776 11.0568 21.5395 11.0568 21.2481V20.5345C11.0568 20.2431 11.2896 20.0066 11.5766 20.0066H12.2793C12.5663 20.0066 12.7991 20.2431 12.7991 20.5345V21.2481C12.7991 21.5395 12.5663 21.776 12.2793 21.776Z"
        fill="white"
      />
      <path
        d="M18.0139 5.29049H17.3102C17.0232 5.29049 16.7914 5.05402 16.7914 4.76259V4.04903C16.7914 3.75759 17.0232 3.52112 17.3102 3.52112H18.0139C18.2999 3.52112 18.5327 3.75759 18.5327 4.04903V4.76259C18.5327 5.05402 18.2999 5.29049 18.0139 5.29049Z"
        fill="white"
      />
    </svg>
  );
};
