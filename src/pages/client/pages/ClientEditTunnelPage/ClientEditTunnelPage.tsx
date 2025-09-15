import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgIconCheckmarkSmall from '../../../../shared/components/svg/IconCheckmarkSmall';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { routes } from '../../../../shared/routes';
import { clientApi } from '../../clientAPI/clientApi';
import { useClientStore } from '../../hooks/useClientStore';
import { clientQueryKeys } from '../../query';
import { ClientConnectionType } from '../../types';
import { EditTunnelFormCard } from './components/EditTunnelFormCard';
import { DeleteTunnelModal } from './modals/DeleteTunnelModal/DeleteTunnelModal';
import { useDeleteTunnelModal } from './modals/DeleteTunnelModal/useDeleteTunnelModal';

const { getTunnelDetails } = clientApi;

export const ClientEditTunnelPage = () => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const submitRef = useRef<HTMLInputElement | null>(null);
  const selectedInstance = useClientStore((state) => state.selectedInstance);
  const openDeleteTunnel = useDeleteTunnelModal((state) => state.open);
  useEffect(() => {
    if (
      selectedInstance?.id === undefined ||
      selectedInstance.type !== ClientConnectionType.TUNNEL
    ) {
      navigate(routes.client.base, { replace: true });
    }
  }, [selectedInstance, navigate]);

  const { data: tunnel } = useQuery({
    queryKey: [clientQueryKeys.getTunnels, selectedInstance?.id as number],
    queryFn: () => getTunnelDetails(selectedInstance?.id as number),
    enabled: !!selectedInstance?.id,
  });
  return (
    <>
      <section className="client-page" id="client-edit-tunnel-page">
        <header>
          <h1>{LL.pages.client.pages.editTunnelPage.title()}</h1>
          <div className="controls">
            <Button
              size={ButtonSize.SMALL}
              styleVariant={ButtonStyleVariant.STANDARD}
              text={LL.common.controls.cancel()}
              type="submit"
              onClick={() => navigate(routes.client.base, { replace: true })}
            />
            <Button
              size={ButtonSize.SMALL}
              styleVariant={ButtonStyleVariant.DELETE}
              text={'Delete tunnel'}
              type="submit"
              onClick={() => {
                if (tunnel) {
                  openDeleteTunnel(tunnel);
                }
              }}
            />
            <Button
              size={ButtonSize.SMALL}
              styleVariant={ButtonStyleVariant.SAVE}
              text={LL.pages.client.pages.editTunnelPage.controls.save()}
              icon={<SvgIconCheckmarkSmall />}
              type="submit"
              onClick={() => submitRef.current?.click()}
            />
          </div>
        </header>
        <div className="content">
          {tunnel && <EditTunnelFormCard tunnel={tunnel} submitRef={submitRef} />}
        </div>
      </section>
      <DeleteTunnelModal />
    </>
  );
};
