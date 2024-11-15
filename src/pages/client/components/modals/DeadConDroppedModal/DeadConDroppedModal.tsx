import './style.scss';

import { useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { ClientConnectionType } from '../../../types';
import { useDeadConDroppedModal } from './store';

export const DeadConDroppedModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.modals.deadConDropped;
  const isOpen = useDeadConDroppedModal((s) => s.visible);
  const payload = useDeadConDroppedModal((s) => s.payload);
  const [close, reset] = useDeadConDroppedModal((s) => [s.close, s.reset], shallow);

  return (
    <ModalWithTitle
      isOpen={isOpen}
      title={localLL.title({
        conType: payload?.con_type ?? '',
      })}
      afterClose={reset}
      onClose={close}
      id="dead-con-dropped-modal"
      className="middle"
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.modals.deadConDropped;
  const payload = useDeadConDroppedModal((s) => s.payload);
  const close = useDeadConDroppedModal((s) => s.close, shallow);

  const typeString = useMemo(() => {
    switch (payload?.con_type) {
      case ClientConnectionType.LOCATION:
        return localLL.location();
      case ClientConnectionType.TUNNEL:
        return localLL.tunnel();
      default:
        return '';
    }
  }, [localLL, payload?.con_type]);

  if (!payload) return null;
  return (
    <>
      <div className="message">
        <p>
          {localLL.body({
            conType: typeString,
            instanceName: payload.interface_name,
          })}
        </p>
      </div>
      <div className="controls">
        <Button
          className="close"
          text={localLL.controls.close()}
          onClick={() => close()}
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.LARGE}
        />
      </div>
    </>
  );
};
