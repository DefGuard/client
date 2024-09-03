import './style.scss';

import { useNotificationStore } from '../../../../../../components/NotificationManager/useNotificationStore';
import { useI18nContext } from '../../../../../../i18n/i18n-react';

export const Notification = () => {
  const { LL } = useI18nContext();
  const [header, text, dismissed, setValues] = useNotificationStore((state) => [
    state.header,
    state.text,
    state.dismissed,
    state.setValues,
  ]);

  if (dismissed) return null;

  return (
    <div id="notification">
      <div className="notification-header">
        <h3>{header}</h3>
      </div>
      <div className="notification-subheader">
        <p onClick={() => setValues({ dismissed: true })}>
          {LL.pages.client.notification.dismiss()}
        </p>
        <p>{text}</p>
      </div>
      <div className="notification-mobile">
        <p>{header}</p>
        <p>{text}</p>
        <div>
          <p onClick={() => setValues({ dismissed: true })}>
            {LL.pages.client.notification.dismiss()}
          </p>
        </div>
      </div>
    </div>
  );
};
