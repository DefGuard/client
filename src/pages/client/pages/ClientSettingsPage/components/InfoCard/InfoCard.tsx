import Markdown from 'react-markdown';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
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
      rightIcon={<GithubIcon />}
      onClick={() => {
        openLink(defguardGithubLink);
      }}
    />
  );
};

export const GithubIcon = () => {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" width={36} height={36} fill="none">
      <g fill="#fff">
        <path
          fillRule="evenodd"
          d="M18.5.75A18.065 18.065 0 0 0 6.84 5.02a17.876 17.876 0 0 0-6.1 10.763 17.816 17.816 0 0 0 2.364 12.13 17.985 17.985 0 0 0 9.703 7.716c.895.166 1.232-.388 1.232-.859s-.018-1.838-.024-3.331c-5.008 1.082-6.066-2.113-6.066-2.113-.817-2.075-1.997-2.62-1.997-2.62-1.633-1.109.122-1.088.122-1.088 1.81.128 2.76 1.847 2.76 1.847 1.604 2.735 4.212 1.944 5.237 1.481.161-1.158.63-1.947 1.145-2.394-4-.45-8.203-1.986-8.203-8.844a6.91 6.91 0 0 1 1.854-4.804c-.185-.45-.802-2.27.176-4.742 0 0 1.511-.48 4.95 1.835a17.18 17.18 0 0 1 9.014 0c3.437-2.315 4.945-1.835 4.945-1.835.98 2.466.364 4.286.179 4.742a6.896 6.896 0 0 1 1.857 4.81c0 6.873-4.212 8.387-8.218 8.829.644.557 1.22 1.645 1.22 3.316 0 2.395-.022 4.321-.022 4.911 0 .477.325 1.034 1.237.86a17.984 17.984 0 0 0 9.705-7.719 17.816 17.816 0 0 0 2.363-12.13 17.877 17.877 0 0 0-6.104-10.764A18.065 18.065 0 0 0 18.506.75H18.5Z"
          clipRule="evenodd"
        />
        <path d="M7.316 26.448c-.038.089-.181.116-.298.053-.116-.062-.202-.178-.16-.27.041-.091.181-.115.297-.053.117.063.206.181.161.27ZM8.047 27.257a.3.3 0 0 1-.37-.083c-.116-.124-.14-.296-.05-.373.089-.077.25-.042.366.083.116.124.143.296.054.373ZM8.756 28.285c-.11.078-.298 0-.402-.154a.296.296 0 0 1 0-.427c.11-.073.298 0 .402.152.105.15.108.352 0 .43ZM9.72 29.281c-.099.11-.299.08-.463-.068-.164-.148-.202-.35-.104-.456.098-.107.298-.077.468.068.17.145.203.35.098.456ZM11.07 29.862c-.045.14-.248.202-.45.142-.204-.059-.338-.225-.299-.367.039-.142.245-.208.45-.142.206.065.337.222.298.367ZM12.541 29.963c0 .145-.166.27-.381.272-.215.003-.39-.115-.39-.26 0-.146.166-.27.381-.273.215-.003.39.113.39.26ZM13.913 29.735c.026.145-.123.296-.337.332-.215.035-.403-.05-.43-.193-.026-.142.129-.296.337-.335.209-.038.403.05.43.196Z" />
      </g>
    </svg>
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
