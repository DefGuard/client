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
import securityImage from '../../../CarouselPage/cards/assets/slide_security.png';

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
    <div className="centered">
      <p className="more">{localLL.isMore()}</p>
      <GithubButton />
    </div>
  );
};

export const InfoCard = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.security;
  const supportLL = LL.pages.client.pages.carouselPage.slides.support;
  return (
    <Card shaded id="security-card">
      <div className="row">
        <img src={securityImage} />
        <div className="text">
          <h1 className="centered">
            <Markdown>{localLL.title()}</Markdown>
          </h1>
          <Markdown>{localLL.sideText()}</Markdown>
          <MoreSection />
          <h1 className="centered"> {supportLL.supportUs()}</h1>
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
      </div>
    </Card>
  );
};
