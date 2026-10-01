import { OpenIdProvider, type OpenIdProviderValue } from '../../../../consts';
import googleWhiteImage from './assets/google-white.svg';
import jumpcloudWhiteImage from './assets/jumpcloud-white.svg';
import microsoftWhiteImage from './assets/microsoft-white.svg';
import oktaWhiteImage from './assets/okta-white.svg';

export const openIdProviderIcon: Record<OpenIdProviderValue, string> = {
  [OpenIdProvider.Microsoft]: microsoftWhiteImage,
  [OpenIdProvider.Google]: googleWhiteImage,
  [OpenIdProvider.Okta]: oktaWhiteImage,
  [OpenIdProvider.JumpCloud]: jumpcloudWhiteImage,
};
