import './style.scss';
import { useMutation, useQuery } from '@tanstack/react-query';
import { error as logError } from '@tauri-apps/plugin-log';
import { openUrl } from '@tauri-apps/plugin-opener';
import { platform } from '@tauri-apps/plugin-os';
import Markdown from 'react-markdown';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Divider } from '../../../shared/components/Divider/Divider';
import { IconKind } from '../../../shared/components/Icon';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { useUpdateAvailable } from '../../../shared/hooks/useUpdateAvailable';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { Snackbar } from '../../../shared/providers/snackbar/snackbar';
import { api } from '../../../shared/rust-api/api';
import { getLatestAppVersionQueryOptions } from '../../../shared/rust-api/query';
import type { NewAppVersionInfo } from '../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import upToDateBannerSrc from './assets/banner_up_to_date.png';
import updateAvailableBannerSrc from './assets/banner_update_available.png';

// The Windows service installs updates itself; elsewhere the user installs them manually.
const canInstallInPlace = platform() === 'windows';

const UpdateAvailable = ({ info }: { info: NewAppVersionInfo }) => {
  const {
    mutate: startUpdate,
    isPending,
    isSuccess,
  } = useMutation({
    mutationFn: api.startUpdate,
    onSuccess: (version) => {
      Snackbar.default(`Installing version ${version}. The app will restart shortly.`);
    },
    onError: (e) => {
      void logError(`Starting update failed: ${e}`);
      Snackbar.error(`Couldn't install the update: ${e}`);
    },
  });

  return (
    <>
      <p className="title">New {info.version} version is available.</p>
      <SizedBox height={ThemeSpacing.Sm} />
      {isPresent(info.summary) && <p className="description">{info.summary}</p>}
      <SizedBox height={ThemeSpacing.Xl} />
      <div className="actions">
        <Button
          text={`Update to ${info.version} version now`}
          {...(canInstallInPlace
            ? { loading: isPending || isSuccess, onClick: () => startUpdate() }
            : {
                iconRight: IconKind.OpenInNewWindow,
                onClick: () => openUrl(info.update_url),
              })}
        />
        {canInstallInPlace && (
          <Button
            text="Download installer"
            variant={ButtonVariant.Secondary}
            iconRight={IconKind.OpenInNewWindow}
            onClick={() => openUrl(info.update_url)}
          />
        )}
        <Button
          text="Read more in Blog"
          variant={ButtonVariant.Secondary}
          iconRight={IconKind.OpenInNewWindow}
          onClick={() => openUrl(info.release_notes_url)}
        />
      </div>
      {isPresent(info.notes) && (
        <>
          <Divider spacing={ThemeSpacing.Xl2} />
          <div className="notes">
            <Markdown>{info.notes}</Markdown>
          </div>
        </>
      )}
    </>
  );
};

const UpToDate = () => (
  <>
    <p className="title">You're up to date!</p>
    <SizedBox height={ThemeSpacing.Sm} />
    <p className="description">
      You are currently using the latest version of the application.
      <br />
      There are no new updates available at this time.
    </p>
  </>
);

export const UpdatePage = () => {
  const { data: latest } = useQuery(getLatestAppVersionQueryOptions);
  const updateAvailable = useUpdateAvailable();

  return (
    <FullPage id="update-page-view" hideScrollContainer>
      <img
        className="banner"
        src={updateAvailable ? updateAvailableBannerSrc : upToDateBannerSrc}
        loading="eager"
      />
      <SizedBox height={ThemeSpacing.Xl} />
      {updateAvailable && latest ? <UpdateAvailable info={latest} /> : <UpToDate />}
    </FullPage>
  );
};
