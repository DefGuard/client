import './style.scss';

import { shallow } from 'zustand/shallow';

import { useApplicationUpdateStore } from '../../../../../../components/ApplicationUpdateManager/useApplicationUpdateStore';
import { useNewAppVersionAvailable } from '../../../../../../components/ApplicationUpdateManager/useNewAppVersionAvailable';
import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { clientApi } from '../../../../../../pages/client/clientAPI/clientApi';
import SvgIconDownload from '../../../../../../shared/defguard-ui/components/svg/IconDownload';
import { useClientStore } from '../../../../../client/hooks/useClientStore';

const { openLink } = clientApi;

export const NewApplicationVersionAvailableInfo = () => {
  const { LL } = useI18nContext();
  const { newAppVersionAvailable } = useNewAppVersionAvailable();
  const checkForUpdates = useClientStore((state) => state.settings.check_for_updates);

  const dismissed = useApplicationUpdateStore((state) => state.dismissed, shallow);
  const setValues = useApplicationUpdateStore((state) => state.setValues, shallow);

  const [latestVersion, releaseDate, releaseNotesUrl, updateUrl] =
    useApplicationUpdateStore(
      (state) => [
        state.latestVersion,
        state.releaseDate,
        state.releaseNotesUrl,
        state.updateUrl,
      ],
      shallow,
    );

  if (
    dismissed ||
    !checkForUpdates ||
    !newAppVersionAvailable ||
    !latestVersion ||
    !releaseDate ||
    !releaseNotesUrl ||
    !updateUrl
  )
    return null;

  return (
    <div id="settings-new-application-version-available">
      <div className="new-version-header">
        <h3>
          {LL.pages.client.newApplicationVersion.header()} {latestVersion}
        </h3>
        <SvgIconDownload
          className="new-version-download-icon"
          onClick={() => openLink(updateUrl)}
        />
      </div>
      <div className="new-version-subheader">
        <p onClick={() => setValues({ dismissed: true })}>
          {LL.pages.client.newApplicationVersion.dismiss()}
        </p>
        <p onClick={() => openLink(releaseNotesUrl)}>
          {LL.pages.client.newApplicationVersion.releaseNotes()}
        </p>
      </div>
      <div className="settings-new-application-version-mobile">
        <p>{LL.pages.client.newApplicationVersion.header()}</p>
        <p>{latestVersion}</p>
        <SvgIconDownload
          className="new-version-download-icon"
          onClick={() => openLink(updateUrl)}
        />
        <div>
          <p onClick={() => openLink(releaseNotesUrl)}>
            {LL.pages.client.newApplicationVersion.releaseNotes()}
          </p>
          <p onClick={() => setValues({ dismissed: true })}>
            {LL.pages.client.newApplicationVersion.dismiss()}
          </p>
        </div>
      </div>
    </div>
  );
};
