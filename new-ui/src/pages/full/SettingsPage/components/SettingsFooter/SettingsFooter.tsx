import './style.scss';
import { openUrl } from '@tauri-apps/plugin-opener';
import { Icon, IconKind } from '../../../../../shared/components/Icon';

export const SettingsFooter = () => {
  return (
    <p className="settings-footer">
      Defguard is made possible by other open-source software.{' '}
      <button
        type="button"
        className="link"
        onClick={() => openUrl('https://docs.defguard.net/')}
      >
        Learn more here
        <Icon icon={IconKind.OpenInNewWindow} size={14} />
      </button>
    </p>
  );
};
