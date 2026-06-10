import './style.scss';
import { openUrl } from '@tauri-apps/plugin-opener';
import type { ReactNode } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Divider } from '../../../shared/components/Divider/Divider';
import { FullPageTitle } from '../../../shared/components/FullPageTitle/FullPageTitle';
import { Icon, IconKind } from '../../../shared/components/Icon';
import type { IconKindValue } from '../../../shared/components/Icon/icon-types';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { ThemeSpacing } from '../../../shared/types';

type SupportSectionProps = {
  icon: IconKindValue;
  title: string;
  description: ReactNode;
  action?: ReactNode;
  divider?: boolean;
};

const SupportSection = ({
  icon,
  title,
  description,
  action,
  divider = true,
}: SupportSectionProps) => (
  <div className="section">
    <div>
      <Icon icon={icon} />
    </div>
    <div className="content">
      <p className="title">{title}</p>
      <SizedBox height={ThemeSpacing.Sm} />
      <p className="description">{description}</p>
      {action && (
        <>
          <SizedBox height={ThemeSpacing.Xl} />
          {action}
        </>
      )}
      {divider && <Divider spacing={ThemeSpacing.Xl} />}
    </div>
  </div>
);

export const SupportPage = () => {
  return (
    <FullPage id="support-page-view">
      <FullPageTitle title="Support" spacing={ThemeSpacing.Xl} />
      <div className="sections">
        <SupportSection
          icon={IconKind.Question}
          title="Have questions? Check our documentation first."
          description="Before contacting or submitting any issues to GitHub please get familiar with Defguard documentation available."
          action={
            <Button
              text={'Go to documentation'}
              iconRight={IconKind.OpenInNewWindow}
              onClick={() => openUrl('https://docs.defguard.net/')}
            />
          }
        />
        <SupportSection
          icon={IconKind.Bug}
          title="Report a bug"
          description="We aim to respond to all bug reports as quickly as possible and prioritize them based on severity before adding them to our development backlog. To give us more context, you can optionally download the support data and/or log file and attach it to your bug report."
          action={
            <Button
              text={'Report on Github'}
              iconLeft={IconKind.Github}
              variant={ButtonVariant.Secondary}
              onClick={() =>
                openUrl(
                  'https://github.com/DefGuard/client/issues/new?template=02-bug.yml',
                )
              }
            />
          }
        />
        <SupportSection
          icon={IconKind.Request}
          title="Request feature"
          description="We grow with the help of our community. If you have an idea or a missing feature to suggest, please share it - we'll review it."
          action={
            <Button
              text={'Submit on Github'}
              iconLeft={IconKind.Github}
              variant={ButtonVariant.Secondary}
              onClick={() =>
                openUrl(
                  'https://github.com/DefGuard/client/issues/new?template=01-feature-request.yml',
                )
              }
            />
          }
        />
        <SupportSection
          icon={IconKind.Mail}
          title="Have questions? Check our documentation first."
          description="You can contact us for any other requests at support@defguard.net."
          divider={false}
        />
      </div>
    </FullPage>
  );
};
