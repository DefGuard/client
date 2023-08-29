import './style.scss';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

export const AdminInfo = () => {
  const { LL } = useI18nContext();
  const adminInfo = useEnrollmentStore((state) => state.adminInfo);

  if (!adminInfo) return null;

  return (
    <div className="admin-info">
      <p className="title">{LL.components.adminInfo.title()}:</p>
      <p>{adminInfo.name}</p>
      {adminInfo.phone_number && <p>{adminInfo.phone_number}</p>}
      <p>{adminInfo.email}</p>
    </div>
  );
};
