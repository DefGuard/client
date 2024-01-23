import Markdown from 'react-markdown';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { GitHubIcon } from '../../../../../../shared/components/svg/GithubIcon';
import { githubUrl, mastodonUrl, matrixUrl } from '../../../../../../shared/constants';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { defguardGithubLink } from '../../../../../../shared/links';
import { clientApi } from '../../../../clientAPI/clientApi';
import cardImage from './assets/hero.png';

const { openLink } = clientApi;

const GithubButton = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.shared;
  return (
    <Button
      className="github"
      size={ButtonSize.LARGE}
      styleVariant={ButtonStyleVariant.PRIMARY}
      text={localLL.githubButton()}
      rightIcon={<GitHubIcon />}
      onClick={() => {
        openLink(defguardGithubLink);
      }}
    />
  );
};

const MoreSection = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.shared;

  return (
    <>
      <Markdown>{localLL.isMore()}</Markdown>
      <GithubButton />
    </>
  );
};

export const InfoCard = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.security;
  const supportLL = LL.pages.client.pages.carouselPage.slides.support;
  return (
    <Card shaded bordered id="security-card">
      <img src={cardImage} />
      <div className="content">
        <div className="content-wrapper title">
          <Markdown>{localLL.title()}</Markdown>
        </div>
        <Markdown>{localLL.sideText()}</Markdown>
        <MoreSection />
        <div className="content-wrapper title">
          <p>{supportLL.supportUs()}</p>
        </div>
        <Markdown>{supportLL.text()}</Markdown>
        <ul>
          <li>
            <span>{supportLL.githubText()} </span>
            <a onClick={() => openLink(githubUrl)}>{supportLL.githubLink()}</a>
          </li>
          <li>
            <span onClick={() => openLink(mastodonUrl)}>
              {supportLL.spreadWordText()}{' '}
            </span>
            <b>{supportLL.defguard()}</b>
          </li>
          <li>
            <span>{supportLL.joinMatrix()} </span>
            <a onClick={() => openLink(matrixUrl)}>{matrixUrl}</a>
          </li>
        </ul>
      </div>
    </Card>
  );
};
