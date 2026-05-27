import { type CSSProperties, type Ref, useMemo } from 'react';
import type { IconKindValue } from './icon-types';
import './style.scss';
import clsx from 'clsx';
import type { DirectionValue, ThemeVariableValue } from '../../types';
import { isPresent } from '../../utils/isPresent';
import { IconAccessSettings } from './icons/IconAccessSettings';
import { IconActivity } from './icons/IconActivity';
import { IconActivityNotes } from './icons/IconActivityNotes';
import { IconAddAlias } from './icons/IconAddAlias';
import { IconAddDevice } from './icons/IconAddDevice';
import { IconAddGroup } from './icons/IconAddGroup';
import { IconAddLocation } from './icons/IconAddLocation';
import { IconAddRule } from './icons/IconAddRule';
import { IconAddToken } from './icons/IconAddToken';
import { IconAddUser } from './icons/IconAddUser';
import { IconAliases } from './icons/IconAliases';
import { IconAndroid } from './icons/IconAndroid';
import { IconApple } from './icons/IconApple';
import { IconAppStore } from './icons/IconAppstore';
import { IconArchLinux } from './icons/IconArchLinux';
import { IconArrowBig } from './icons/IconArrowBig';
import { IconArrowSmall } from './icons/IconArrowSmall';
import { IconAttentionFilled } from './icons/IconAttentionFilled';
import { IconAttentionOutlined } from './icons/IconAttentionOutlined';
import { IconAuthorisedApp } from './icons/IconAuthorisedApp';
import { IconBiometric } from './icons/IconBiometric';
import { IconBug } from './icons/IconBug';
import { IconCalendar } from './icons/IconCalendar';
import { IconChat } from './icons/IconChat';
import { IconCheck } from './icons/IconCheck';
import { IconCheckCircle } from './icons/IconCheckCircle';
import { IconCheckFilled } from './icons/IconCheckFilled';
import { IconClear } from './icons/IconClear';
import { IconClose } from './icons/IconClose';
import { IconCode } from './icons/IconCode';
import { IconConfig } from './icons/IconConfig';
import { IconConnectedDevices } from './icons/IconConnectedDevices';
import { IconCopy } from './icons/IconCopy';
import { IconCreditCard } from './icons/IconCreditCard';
import { IconCustomize } from './icons/IconCustomize';
import { IconDarkTheme } from './icons/IconDarkTheme';
import { IconDebian } from './icons/IconDebian';
import { IconDelete } from './icons/IconDelete';
import { IconDeploy } from './icons/IconDeploy';
import { IconDesktop } from './icons/IconDesktop';
import { IconDevices } from './icons/IconDevices';
import { IconDevicesActive } from './icons/IconDevicesActive';
import { IconDisabled } from './icons/IconDisabled';
import { IconDisableMfa } from './icons/IconDisableMfa';
import { IconDisconnectAll } from './icons/IconDisconnectAll';
import { IconDownload } from './icons/IconDownload';
import { IconEdit } from './icons/IconEdit';
import { IconEmptyPoint } from './icons/IconEmptyPoint';
import { IconEnrollment } from './icons/IconEnrollment';
import { IconEnter } from './icons/IconEnter';
import { IconExternalMfa } from './icons/IconExternalMFA';
import { IconFile } from './icons/IconFile';
import { IconFileAdd } from './icons/IconFileAdd';
import { IconFiltration } from './icons/IconFiltration';
import { IconGateway } from './icons/IconGateway';
import { IconGithub } from './icons/IconGithub';
import { IconGlobe } from './icons/IconGlobe';
import { IconGlobeBlocked } from './icons/IconGlobeBlocked';
import { IconGroups } from './icons/IconGroups';
import { IconHamburger } from './icons/IconHamburger';
import { IconHelp } from './icons/IconHelp';
import { IconHide } from './icons/IconHide';
import { IconInfoFilled } from './icons/IconInfoFilled';
import { IconInfoOutlined } from './icons/IconInfoOutlined';
import { IconInternalMfa } from './icons/IconInternalMFA';
import { IconIpSuggest } from './icons/IconIpSuggest';
import { IconKey } from './icons/IconKey';
import { IconLightBulb } from './icons/IconLightBulb';
import { IconLightTheme } from './icons/IconLightTheme';
import { IconLinux } from './icons/IconLinux';
import { IconLoader } from './icons/IconLoader';
import { IconLocation } from './icons/IconLocation';
import { IconLocationTracking } from './icons/IconLocationTracking';
import { IconLockOpen } from './icons/IconLock';
import { IconLockClosed } from './icons/IconLockClosed';
import { IconLogout } from './icons/IconLogout';
import { IconMail } from './icons/IconMail';
import { IconMenu } from './icons/IconMenu';
import { IconMinusCircle } from './icons/IconMinusCircle';
import { IconMobile } from './icons/IconMobile';
import { IconMobileLock } from './icons/IconMobileLock';
import { IconNetworkSettings } from './icons/IconNetworkSettings';
import { IconNotification } from './icons/IconNotification';
import { IconOneTimePassword } from './icons/IconOneTimePassword';
import { IconOnline } from './icons/IconOnline';
import { IconOpenId } from './icons/IconOpenId';
import { IconOpenInNewWindow } from './icons/IconOpenInNewWindow';
import { IconPending } from './icons/IconPending';
import { IconPieChart } from './icons/IconPieChart';
import { IconPlay } from './icons/IconPlay';
import { IconPlayFilled } from './icons/IconPlayFilled';
import { IconPlus } from './icons/IconPlus';
import { IconPlusCircle } from './icons/IconPlusCircle';
import { IconProfile } from './icons/IconProfile';
import { IconProtection } from './icons/IconProtection';
import { IconRefresh } from './icons/IconRefresh';
import { IconRequest } from './icons/IconRequest';
import { IconRules } from './icons/IconRules';
import { IconSearch } from './icons/IconSearch';
import { IconServers } from './icons/IconServers';
import { IconSettings } from './icons/IconSettings';
import { IconShow } from './icons/IconShow';
import { IconSortable } from './icons/IconSortable';
import { IconStatusAttention } from './icons/IconStatusAttention';
import { IconStatusAvailable } from './icons/IconStatusAvailable';
import { IconStatusImportant } from './icons/IconStatusImportant';
import { IconStatusPremium } from './icons/IconStatusPremium';
import { IconStatusSimple } from './icons/IconStatusSimple';
import { IconSupport } from './icons/IconSupport';
import { IconSync } from './icons/IconSync';
import { IconToken } from './icons/IconToken';
import { IconTransactions } from './icons/IconTransactions';
import { IconTutorial } from './icons/IconTutorial';
import { IconTutorialNotAvailable } from './icons/IconTutorialNotAvailable';
import { IconUbuntu } from './icons/IconUbuntu';
import { IconUpload } from './icons/IconUpload';
import { IconUser } from './icons/IconUser';
import { IconUserActive } from './icons/IconUserActive';
import { IconUsers } from './icons/IconUsers';
import { IconWarningFilled } from './icons/IconWarningFilled';
import { IconWarningOutlined } from './icons/IconWarningOutlined';
import { IconWebhooks } from './icons/IconWebhooks';
import { IconWindows } from './icons/IconWindows';

type Props<T extends IconKindValue = IconKindValue> = {
  icon: T;
  staticColor?: ThemeVariableValue;
  size?: number;
  rotationDirection?: DirectionValue;
  customRotation?: number;
  ref?: Ref<HTMLDivElement>;
  className?: string;
};

type RotationMap = Record<DirectionValue, number>;

const mapRotation = (kind: IconKindValue, direction: DirectionValue): number => {
  switch (kind) {
    case 'arrow-small':
    case 'arrow-big': {
      const map: RotationMap = {
        down: 90,
        right: 0,
        up: -90,
        left: 180,
      };
      return map[direction];
    }
  }
  console.error(`Unimplemented rotation mapping for icon kind of ${kind}`);
  // safe return for unimplemented
  return 0;
};

const EmptyIcon = () => {
  return null;
};

// Color should be set by css bcs some icons have different structures like 'loader'
export const Icon = <T extends IconKindValue>({
  icon: iconKind,
  rotationDirection,
  customRotation,
  ref,
  className,
  staticColor,
  size,
}: Props<T>) => {
  const IconToRender = useMemo(() => {
    switch (iconKind) {
      case 'mobile-lock':
        return IconMobileLock;
      case 'sync':
        return IconSync;
      case 'attention-filled':
        return IconAttentionFilled;
      case 'ip-suggest':
        return IconIpSuggest;
      case 'filtration':
        return IconFiltration;
      case 'rules':
        return IconRules;
      case 'add-rule':
        return IconAddRule;
      case 'add-alias':
        return IconAddAlias;
      case 'aliases':
        return IconAliases;
      case 'upload':
        return IconUpload;
      case 'lock-closed':
        return IconLockClosed;
      case 'enrollment':
        return IconEnrollment;
      case 'customize':
        return IconCustomize;
      case 'light-theme':
        return IconLightTheme;
      case 'dark-theme':
        return IconDarkTheme;
      case 'refresh':
        return IconRefresh;
      case 'network-settings':
        return IconNetworkSettings;
      case 'connected-devices':
        return IconConnectedDevices;
      case 'external-mfa':
        return IconExternalMfa;
      case 'internal-mfa':
        return IconInternalMfa;
      case 'token':
        return IconToken;
      case 'add-location':
        return IconAddLocation;
      case 'add-group':
        return IconAddGroup;
      case 'add-token':
        return IconAddToken;
      case 'online':
        return IconOnline;
      case 'key':
        return IconKey;
      case 'add-device':
        return IconAddDevice;
      case 'warning-filled':
        return IconWarningFilled;
      case 'warning-outlined':
        return IconWarningOutlined;
      case 'ubuntu':
        return IconUbuntu;
      case 'debian':
        return IconDebian;
      case 'arch-linux':
        return IconArchLinux;
      case 'disabled':
        return IconDisabled;
      case 'disable-mfa':
        return IconDisableMfa;
      case 'show':
        return IconShow;
      case 'hide':
        return IconHide;
      case 'copy':
        return IconCopy;
      case 'config':
        return IconConfig;
      case 'open-in-new-window':
        return IconOpenInNewWindow;
      case 'arrow-big':
        return IconArrowBig;
      case 'arrow-small':
        return IconArrowSmall;
      case 'loader':
        return IconLoader;
      case 'plus':
        return IconPlus;
      case 'status-simple':
        return IconStatusSimple;
      case 'lock-open':
        return IconLockOpen;
      case 'check-circle':
        return IconCheckCircle;
      case 'check-filled':
        return IconCheckFilled;
      case 'empty-point':
        return IconEmptyPoint;
      case 'desktop':
        return IconDesktop;
      case 'mobile':
        return IconMobile;
      case 'windows':
        return IconWindows;
      case 'linux':
        return IconLinux;
      case 'app-store':
        return IconAppStore;
      case 'apple':
        return IconApple;
      case 'android':
        return IconAndroid;
      case 'close':
        return IconClose;
      case 'file':
        return IconFile;
      case 'file-add':
        return IconFileAdd;
      case 'globe':
        return IconGlobe;
      case 'globe-blocked':
        return IconGlobeBlocked;
      case 'help':
        return IconHelp;
      case 'access-settings':
        return IconAccessSettings;
      case 'activity':
        return IconActivity;
      case 'activity-notes':
        return IconActivityNotes;
      case 'add-user':
        return IconAddUser;
      case 'analytics':
        return EmptyIcon;
      case 'archive':
        return EmptyIcon;
      case 'attention-outlined':
        return IconAttentionOutlined;
      case 'check':
        return IconCheck;
      case 'clear':
        return IconClear;
      case 'code':
        return IconCode;
      case 'collapse':
        return EmptyIcon;
      case 'credit-card':
        return IconCreditCard;
      case 'date':
        return EmptyIcon;
      case 'delete':
        return IconDelete;
      case 'deploy':
        return IconDeploy;
      case 'devices':
        return IconDevices;
      case 'devices-active':
        return IconDevicesActive;
      case 'download':
        return IconDownload;
      case 'edit':
        return IconEdit;
      case 'enter':
        return IconEnter;
      case 'expand':
        return EmptyIcon;
      case 'filter':
        return EmptyIcon;
      case 'gateway':
        return IconGateway;
      case 'gift':
        return EmptyIcon;
      case 'github':
        return IconGithub;
      case 'groups':
        return IconGroups;
      case 'hamburger':
        return IconHamburger;
      case 'info-filled':
        return IconInfoFilled;
      case 'info-outlined':
        return IconInfoOutlined;
      case 'location':
        return IconLocation;
      case 'location-preview':
        return EmptyIcon;
      case 'location-tracking':
        return IconLocationTracking;
      case 'logout':
        return IconLogout;
      case 'mail':
        return IconMail;
      case 'manage-keys':
        return EmptyIcon;
      case 'menu':
        return IconMenu;
      case 'minus-circle':
        return IconMinusCircle;
      case 'navigation-collapse':
        return EmptyIcon;
      case 'navigation-uncollapse':
        return EmptyIcon;
      case 'notification':
        return IconNotification;
      case 'one-time-password':
        return IconOneTimePassword;
      case 'openid':
        return IconOpenId;
      case 'pdf':
        return EmptyIcon;
      case 'pie-chart':
        return IconPieChart;
      case 'plus-circle':
        return IconPlusCircle;
      case 'profile':
        return IconProfile;
      case 'protection':
        return IconProtection;
      case 'qr':
        return EmptyIcon;
      case 'search':
        return IconSearch;
      case 'servers':
        return IconServers;
      case 'settings':
        return IconSettings;
      case 'sort':
        return EmptyIcon;
      case 'sortable':
        return IconSortable;
      case 'status-premium':
        return IconStatusPremium;
      case 'status-attention':
        return IconStatusAttention;
      case 'status-available':
        return IconStatusAvailable;
      case 'status-important':
        return IconStatusImportant;
      case 'support':
        return IconSupport;
      case 'transactions':
        return IconTransactions;
      case 'user':
        return IconUser;
      case 'user-active':
        return IconUserActive;
      case 'users':
        return IconUsers;
      case 'webhooks':
        return IconWebhooks;
      case 'yubi-keys':
        return EmptyIcon;
      case 'biometric':
        return IconBiometric;
      case 'pending':
        return IconPending;
      case 'bug':
        return IconBug;
      case 'chat':
        return IconChat;
      case 'request':
        return IconRequest;
      case 'calendar':
        return IconCalendar;
      case 'light-bulb':
        return IconLightBulb;
      case 'tutorial':
        return IconTutorial;
      case 'tutorial-not-available':
        return IconTutorialNotAvailable;
      case 'authorised-app':
        return IconAuthorisedApp;
      case 'play':
        return IconPlay;
      case 'play-filled':
        return IconPlayFilled;
      case 'disconnect-all':
        return IconDisconnectAll;
    }
  }, [iconKind]);

  const getStyle = useMemo((): CSSProperties => {
    const styles: CSSProperties = {};
    if (isPresent(staticColor)) {
      // @ts-expect-error
      styles['--icon-color'] = staticColor;
    }
    const transform: string[] = [];
    // kind specific configurations
    switch (iconKind) {
      case 'arrow-big':
      case 'arrow-small':
        if (rotationDirection) {
          transform.push(`rotate(${mapRotation(iconKind, rotationDirection)}deg)`);
        }
        break;
    }
    if (customRotation && !rotationDirection) {
      transform.push(`rotate(${customRotation}deg)`);
    }
    if (size) {
      styles.width = size;
      styles.height = size;
    }
    if (transform.length) {
      styles.transform = transform.join(' ');
    }
    return styles;
  }, [iconKind, size, rotationDirection, customRotation, staticColor]);

  return (
    <div
      className={clsx('icon', className)}
      ref={ref}
      style={getStyle}
      data-kind={iconKind}
    >
      <IconToRender />
    </div>
  );
};
