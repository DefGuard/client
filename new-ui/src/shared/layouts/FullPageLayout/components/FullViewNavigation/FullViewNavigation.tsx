import { Link, type LinkProps } from '@tanstack/react-router';
import { Icon, IconKind } from '../../../../components/Icon';
import type { IconKindValue } from '../../../../components/Icon/icon-types';
import './style.scss';

type NavItemDef = LinkProps & {
  icon: IconKindValue;
};

const TOP_LINKS: NavItemDef[] = [
  {
    icon: IconKind.PlusCircle,
    to: '/full/add',
  },
];

const BOTTOM_LINKS: NavItemDef[] = [
  {
    icon: IconKind.Report,
    to: '/full/support',
  },
];

export const FullViewNavigation = () => {
  return (
    <div id="navigation">
      <div className="track">
        <div className="top">
          {TOP_LINKS.map((item, i) => (
            <NavItem key={i} {...item} />
          ))}
        </div>
        <div className="bottom">
          {BOTTOM_LINKS.map((item, i) => (
            <NavItem key={i} {...item} />
          ))}
        </div>
      </div>
    </div>
  );
};

type NavItemProps = NavItemDef;

const NavItem = ({ icon, ...linkProps }: NavItemProps) => {
  return (
    <Link {...linkProps}>
      <Icon icon={icon} size={20} />
    </Link>
  );
};
