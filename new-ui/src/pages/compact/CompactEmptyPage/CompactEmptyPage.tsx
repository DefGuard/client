import './style.scss';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonSize, ButtonVariant } from '../../../shared/components/Button/types';
import { Icon, IconKind } from '../../../shared/components/Icon';
import { WindowHeader } from '../../../shared/components/WindowHeader/WindowHeader';
import { api } from '../../../shared/rust-api/api';
import { CompactPage } from '../CompactPage/CompactPage';

export const CompactEmptyPage = () => {
  return (
    <CompactPage containerProps={{ id: 'compact-empty-page' }}>
      <WindowHeader variant="compact" />
      <div className="empty-card">
        <div className="content">
          <Icon icon={IconKind.DisconnectAll} size={26} />
          <p>{`You don't have any instances or tunnels yet. Click the button below to open Defguard.`}</p>
          <Button
            text="Open Defguard"
            variant={ButtonVariant.Primary}
            size={ButtonSize.Primary}
            onClick={() => {
              void api.swapToOldUi();
            }}
          />
        </div>
      </div>
    </CompactPage>
  );
};
