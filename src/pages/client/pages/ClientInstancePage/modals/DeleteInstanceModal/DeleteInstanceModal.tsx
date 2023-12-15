import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';

export const DeleteInstanceModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage;
  return <ConfirmModal type={ConfirmModalType.WARNING} />;
};
