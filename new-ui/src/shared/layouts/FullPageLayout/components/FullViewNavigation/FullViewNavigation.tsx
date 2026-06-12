import { Link, type LinkProps } from '@tanstack/react-router';
import { useMemo } from 'react';
import { Icon, IconKind } from '../../../../components/Icon';
import type { IconKindValue } from '../../../../components/Icon/icon-types';
import { useAppData } from '../../../../providers/AppDataContext';
import './style.scss';

type NavItemDef = LinkProps & {
  icon: IconKindValue;
  hidden?: boolean;
};

const BOTTOM_LINKS: NavItemDef[] = [
  {
    icon: IconKind.Report,
    to: '/full/support',
  },
];

export const FullViewNavigation = () => {
  const { isEmpty } = useAppData();

  const topLinks: NavItemDef[] = useMemo(
    (): NavItemDef[] => [
      {
        icon: IconKind.Analytics,
        to: '/full/overview',
        hidden: isEmpty,
      },
      {
        icon: IconKind.PlusCircle,
        to: '/full/add',
      },
      {
        icon: IconKind.ActivityNotes,
        to: '/full/log',
      },
    ],
    [isEmpty],
  );

  return (
    <div id="navigation">
      <div className="track">
        <div className="top">
          {topLinks
            .filter((i) => !i.hidden)
            .map((item, i) => (
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

const NavItem = ({ icon, hidden, ...linkProps }: NavItemProps) => {
  if (hidden) return null;
  return (
    <Link {...linkProps}>
      <Icon icon={icon} size={20} />
    </Link>
  );
};
