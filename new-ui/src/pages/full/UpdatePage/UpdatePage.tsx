import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { openUrl } from '@tauri-apps/plugin-opener';
import Markdown from 'react-markdown';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Divider } from '../../../shared/components/Divider/Divider';
import { IconKind } from '../../../shared/components/Icon';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { useUpdateAvailable } from '../../../shared/hooks/useUpdateAvailable';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { getLatestAppVersionQueryOptions } from '../../../shared/rust-api/query';
import type { NewAppVersionInfo } from '../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import upToDateBannerSrc from './assets/banner_up_to_date.png';
import updateAvailableBannerSrc from './assets/banner_update_available.png';

const UpdateAvailable = ({ info }: { info: NewAppVersionInfo }) => (
  <>
    <p className="title">New {info.version} version is available.</p>
    <SizedBox height={ThemeSpacing.Sm} />
    <p className="description">
      We're preparing a new version of the desktop VPN client with a focus on performance,
      stability, and everyday usability. These improvements are based on user feedback and
      internal testing to make your connection faster, safer, and easier to manage.
    </p>
    <SizedBox height={ThemeSpacing.Xl} />
    <div className="actions">
      <Button
        text={`Update to ${info.version} version now`}
        iconRight={IconKind.OpenInNewWindow}
        onClick={() => openUrl(info.update_url)}
      />
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
    <FullPage id="update-page-view">
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
