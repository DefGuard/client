import './style.scss';

import Markdown from 'react-markdown';
import { useNavigate } from 'react-router-dom';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { IconDefguard } from '../../../../../shared/components/icons/IconDefguard/IconDeguard';
import SvgDefguardLogoText from '../../../../../shared/components/svg/DefguardLogoText';
import { GitHubIcon } from '../../../../../shared/components/svg/GithubIcon';
import { githubUrl, mastodonUrl, matrixUrl } from '../../../../../shared/constants';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { defguardGithubLink } from '../../../../../shared/links';
import { routes } from '../../../../../shared/routes';
import { clientApi } from '../../../clientAPI/clientApi';
import twoFactorImage from './assets/slide_2fa.png';
import instancesImage from './assets/slide_instances.png';
import securityImage from './assets/slide_security.png';

const { openLink } = clientApi;

export const WelcomeCardSlide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.welcome;
  const navigate = useNavigate();

  return (
    <Card shaded id="welcome-slide">
      <h2>
        <Markdown>{localLL.title()}</Markdown>
      </h2>
      <div className="row between">
        <div
          className="inner-card"
          onClick={() => navigate(routes.client.addInstance, { replace: true })}
        >
          <h3>{localLL.instance.title()}</h3>
          <p>{localLL.instance.subtitle()}</p>
          <div className="logo-container">
            <IconDefguard />
            <SvgDefguardLogoText />
          </div>
        </div>
        <div
          className="inner-card"
          onClick={() => navigate(routes.client.addTunnel, { replace: true })}
        >
          <h3>{localLL.tunnel.title()}</h3>
          <p>{localLL.tunnel.subtitle()}</p>
          <WireguardIcon />
        </div>
      </div>
    </Card>
  );
};

export const TwoFaSlide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.twoFa;
  return (
    <Card shaded id="factor-slide">
      <h2>
        <Markdown>{localLL.title()}</Markdown>
      </h2>
      <div className="row">
        <img src={twoFactorImage} />
        <div className="text centered">
          <Markdown>{localLL.sideText()}</Markdown>
        </div>
      </div>
      <GithubButton />
    </Card>
  );
};

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
      <div className="more">
        <Markdown>{localLL.isMore()}</Markdown>
      </div>
      <GithubButton />
    </>
  );
};

export const SecuritySlide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.security;

  return (
    <Card shaded id="security-slide">
      <h2>
        <Markdown>{localLL.title()}</Markdown>
      </h2>
      <div className="row">
        <img src={securityImage} />
        <div className="text">
          <Markdown>{localLL.sideText()}</Markdown>
        </div>
      </div>
      <MoreSection />
    </Card>
  );
};

export const InstancesSlide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.instances;

  return (
    <Card shaded id="instances-slide">
      <h2>
        <Markdown>{localLL.title()}</Markdown>
      </h2>
      <div className="row">
        <img src={instancesImage} />
        <div className="text centered">
          <Markdown>{localLL.sideText()}</Markdown>
        </div>
      </div>
      <MoreSection />
    </Card>
  );
};

export const SupportSlide = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.carouselPage.slides.support;
  return (
    <Card shaded id="support-slide">
      <h2>
        <Markdown>{localLL.title()}</Markdown>
      </h2>
      <div className="logo-container">
        <IconDefguard width={56} />
        <SvgDefguardLogoText width={220} height={64} />
      </div>
      <div className="text centered">
        <Markdown>{localLL.text()}</Markdown>
        <ul>
          <li>
            <span>{localLL.githubText()} </span>
            <a onClick={() => openLink(githubUrl)}>{localLL.githubLink()}</a>
          </li>
          <li>
            <span onClick={() => openLink(mastodonUrl)}>{localLL.spreadWordText()} </span>
            <b>{localLL.defguard()}</b>
          </li>
          <li>
            <span>{localLL.joinMatrix()} </span>
            <a onClick={() => openLink(matrixUrl)}>{matrixUrl}</a>
          </li>
        </ul>
      </div>
      <GithubButton />
    </Card>
  );
};

const WireguardIcon = () => {
  return (
    <svg
      className="wireguard-logo"
      xmlns="http://www.w3.org/2000/svg"
      width={213}
      height={36}
      fill="none"
    >
      <path
        fill="#222"
        d="M21.072 3.967a.168.168 0 0 0-.066.207c.01.024.025.046.045.063a.254.254 0 0 0 .348.094l.22-.116.117-.063-.094-.081a10.782 10.782 0 0 0-.176-.15c-.15-.124-.273-.046-.394.046Z"
      />
      <path
        fill="#222"
        fillRule="evenodd"
        d="M16.869.9c18.138 0 17.35 16.594 17.35 16.594S35.722 35.1 17.24 35.1C-2.034 35.1.326 16.818.326 16.818S.827.9 16.868.9Zm4.305 13.13c-1.63-3.148-5.977-4.428-9.38-2.335-4.376 2.692-4.118 8.874-.112 11.338.308.19.447.167.65-.107a7.26 7.26 0 0 1 2.329-2.037c.15-.084.3-.164.473-.254l.087-.046.191-.101c-1.992-.348-3.057-1.262-3.181-2.674-.133-1.52.605-2.755 1.952-3.27a3.018 3.018 0 0 1 1.925-.052 3.035 3.035 0 0 1 2.083 3.512c-.163.87-.677 1.5-1.322 2.054 2.188-.514 3.782-1.726 4.457-3.887.196-.626.157-1.545-.152-2.141ZM7.47 27.337c.444-.154.88-.306 1.326-.42.452-.115.913-.193 1.39-.274.214-.036.43-.073.652-.114a5.592 5.592 0 0 1 1.047-2.951c-2.15-.23-4.692 1.78-5.014 3.964.202-.067.401-.136.599-.205ZM23.58 5.621a36.57 36.57 0 0 1-1.091-.019c-.078-.003-.153-.058-.228-.114a1.223 1.223 0 0 0-.105-.072c.037-.015.074-.034.11-.053.08-.04.16-.08.24-.081a247.97 247.97 0 0 1 2.555-.007h.74c.001-.574-.762-1.36-1.44-1.573l-.015.227c-.674.016-1.336.004-1.937-.318-.108-.058-.19-.164-.273-.27-.038-.05-.077-.099-.117-.143a3.262 3.262 0 0 1-.146-.175c-.106-.133-.211-.265-.348-.345-.262-.153-.54-.28-.819-.405a16.425 16.425 0 0 1-.452-.21c-1.503-.736-3.09-.71-4.796-.554l2.86.67-.03.169c-.842.113-1.652-.063-2.464-.24a15.444 15.444 0 0 0-1.137-.217c.432.258.878.493 1.335.702.323.146.652.276.983.406.146.058.293.115.439.175-.608.522-1.218.637-1.983.461a4.773 4.773 0 0 0-1.287-.126c-.456.012-.9.15-1.284.4.125.064.248.125.368.184.29.144.564.28.828.435.153.09.33.244.372.403.086.387.15.778.192 1.172-.702.08-1.936.798-2.185 1.264.353.069.716.088 1.08.107.75.039 1.509.078 2.201.548-.176.134-.474.285-.76.43-.264.135-.518.263-.657.37.357.094 1.187.047 1.511.025.273-.018.4-.025.511.068l3.175 2.499c.334.27 1.682 1.553 2.034 2.36.2.445.314.925.336 1.413 0 .565-.104 1.124-.309 1.65-.11.28-.432.901-1.097 1.626-1.03 1.122-2.356 1.73-3.806 2.03-3.37.699-6.172 4.318-5.381 8.309.923 4.658 6.037 7.18 10.216 4.964 2.701-1.432 4.133-4.226 3.75-7.268-.232-1.838-1.059-3.337-2.445-4.542-.181-.157-.296-.157-.509-.02-.722.463-1.458.902-2.208 1.317-.292.161-.6.293-.935.436h-.002c-.16.069-.325.14-.498.217l.17.044.193.05c2.015.54 3.092 2.318 2.615 4.31-.424 1.773-2.213 2.906-3.947 2.607-1.445-.25-2.707-1.456-2.918-2.902-.23-1.576.551-3.092 1.94-3.727.37-.169.744-.327 1.118-.486.406-.171.812-.343 1.212-.528.23-.107.466-.208.702-.31.658-.28 1.318-.563 1.88-.978 1.892-1.398 3.062-3.323 3.517-5.646.273-1.391.255-2.777-.378-4.098-.486-1.014-1.283-1.75-2.14-2.423a44.818 44.818 0 0 0-1.3-.97c-.468-.34-.938-.683-1.393-1.043-.237-.189-.397-.514-.507-.808-.047-.125.105-.464.206-.482.54-.094 1.087-.15 1.636-.168.497-.02.996-.01 1.494-.002l.404.006h.073c.124-.001.264-.002.33.062.327.324.584.116.812-.097.177-.193.338-.4.48-.62a3.278 3.278 0 0 0-.472-.066c-.273-.006-.546-.008-.819-.01Z"
        clipRule="evenodd"
      />
      <path
        fill="#222"
        d="M132.26 23.507v-3.345h-6.182v-2.623h8.996v6.856c-1.096 1.9-2.523 3.356-4.282 4.368-1.76 1.012-3.756 1.518-5.987 1.518-3.396 0-6.2-1.126-8.411-3.377-2.211-2.25-3.317-5.104-3.317-8.56 0-3.469 1.109-6.329 3.327-8.58 2.219-2.25 5.019-3.376 8.401-3.376 2.095 0 4.005.46 5.73 1.383a11.148 11.148 0 0 1 4.21 3.945l-2.362 1.693a7.775 7.775 0 0 0-3.08-3.201 8.871 8.871 0 0 0-4.498-1.157c-2.547 0-4.656.878-6.326 2.633-1.671 1.756-2.506 3.976-2.506 6.66 0 2.685.835 4.902 2.506 6.65 1.67 1.749 3.779 2.623 6.326 2.623 1.588 0 3.002-.345 4.241-1.033 1.239-.688 2.31-1.714 3.214-3.077ZM38.498 6.883l7.394 22.798h2.075l5.915-18.358 5.977 18.358h2.053l7.374-22.798h-2.69l-5.649 17.553-5.75-17.553H52.67l-5.772 17.594-5.627-17.594h-2.773ZM68.78 12.004V29.68h2.793V12.004H68.78Z"
      />
      <path
        fill="#222"
        fillRule="evenodd"
        d="M78.048 21.979v7.703h-2.752V12.087h12.077c1.794 0 3.197.434 4.21 1.301 1.014.867 1.52 2.072 1.52 3.614a4.777 4.777 0 0 1-1.201 3.253c-.801.915-1.845 1.476-3.132 1.683l4.867 7.744H90.66l-4.97-7.703h-7.64Zm0-2.582h9.325c.945 0 1.667-.206 2.167-.62.5-.412.75-1.004.75-1.775 0-.771-.25-1.36-.75-1.766-.5-.406-1.223-.609-2.167-.61h-9.325v4.771Z"
        clipRule="evenodd"
      />
      <path
        fill="#222"
        d="M95.698 12.046V29.68h-.001 17.212v-2.54h-14.46v-5.575h8.997v-2.54H98.45v-4.399h13.699v-2.581H95.698ZM140.897 12.087h-2.753v12.019c0 2.189.671 3.758 2.013 4.708 1.342.95 3.546 1.425 6.614 1.425 3.08 0 5.302-.482 6.664-1.445 1.363-.964 2.044-2.527 2.044-4.688V12.087h-2.732v11.296c0 1.694-.397 2.836-1.191 3.428-.794.592-2.376.888-4.744.888-2.356 0-3.93-.296-4.724-.888-.794-.592-1.191-1.735-1.191-3.428V12.087Z"
      />
      <path
        fill="#222"
        fillRule="evenodd"
        d="m155.049 29.681 8.914-17.635h1.725l8.996 17.635h-2.937l-2.28-4.481h-9.263l-2.259 4.481h-2.896Zm6.347-6.815h6.86l-3.41-6.753-3.45 6.753ZM177.736 29.681V21.98h7.641l4.97 7.703h2.978l-4.867-7.744c1.287-.207 2.331-.768 3.132-1.683a4.775 4.775 0 0 0 1.201-3.253c0-1.542-.506-2.747-1.52-3.614-1.013-.867-2.416-1.301-4.21-1.301h-12.077v17.595h2.752Zm9.325-10.284h-9.325v-4.77h9.325c.944 0 1.667.203 2.167.61.499.405.749.994.749 1.765 0 .77-.25 1.363-.749 1.776-.5.413-1.223.62-2.167.62ZM203.212 12.046c2.862 0 5.169.822 6.922 2.467 1.752 1.646 2.629 3.783 2.629 6.412 0 2.658-.859 4.781-2.578 6.371-1.718 1.59-4.043 2.386-6.973 2.386h-7.784V12.046h7.784Zm.041 2.54h-5.073v12.556h5.073c2.081 0 3.718-.561 4.909-1.684 1.191-1.121 1.787-2.646 1.787-4.574 0-1.858-.616-3.373-1.849-4.543-1.232-1.17-2.848-1.755-4.847-1.755Z"
        clipRule="evenodd"
      />
      <path
        fill="#222"
        d="M210.493 12.678v-1.646h.558c.381 0 .553.205.553.406 0 .214-.181.433-.453.483l.558.757h-.268l-.617-.89a.91.91 0 0 0 .146.02c.276 0 .43-.183.43-.343 0-.132-.104-.264-.317-.264h-.386v1.477h-.204Z"
      />
      <path
        fill="#222"
        fillRule="evenodd"
        d="M210.979 10.444h.14c.816 0 1.333.684 1.365 1.409v.082c0 .693-.426 1.368-1.401 1.395h-.064c-.938 0-1.383-.702-1.383-1.404 0-.725.485-1.473 1.343-1.482Zm.059.196c-.35 0-.649.114-.935.424-.195.237-.226.593-.226.593a1.567 1.567 0 0 0-.014.187c0 .647.453 1.295 1.165 1.295l.028-.003a.258.258 0 0 1 .027-.002c.562 0 1.048-.392 1.143-.953a1.33 1.33 0 0 0 .027-.269c0-.611-.449-1.245-1.07-1.272h-.145Z"
        clipRule="evenodd"
      />
    </svg>
  );
};
