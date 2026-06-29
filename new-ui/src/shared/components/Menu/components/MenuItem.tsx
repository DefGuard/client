import {
  autoUpdate,
  FloatingPortal,
  offset,
  safePolygon,
  shift,
  useDismiss,
  useFloating,
  useHover,
  useInteractions,
} from '@floating-ui/react';
import clsx from 'clsx';
import { useState } from 'react';
import { isPresent } from '../../../utils/isPresent';
import { Icon } from '../../Icon';
import { Menu } from '../Menu';
import type { MenuItemProps } from '../types';

export const MenuItem = ({
  disabled,
  text,
  icon,
  items,
  testId,
  variant,
  onClick,
  onClose,
}: MenuItemProps) => {
  const hasItems = isPresent(items) && items.length > 0;
  const hasIcon = isPresent(icon);

  const [submenuOpen, setSubmenuOpen] = useState(false);

  const { refs, context, floatingStyles } = useFloating({
    placement: 'right-start',
    open: submenuOpen,
    onOpenChange: setSubmenuOpen,
    whileElementsMounted: autoUpdate,
    middleware: [offset(12), shift({ padding: 4 })],
  });

  const hover = useHover(context, {
    handleClose: safePolygon(),
    enabled: hasItems && !disabled,
  });

  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    outsidePress: true,
  });

  const { getReferenceProps, getFloatingProps } = useInteractions([hover, dismiss]);

  return (
    <>
      <div
        ref={refs.setReference}
        className={clsx('menu-item', `variant-${variant}`, {
          disabled,
          'grid-default': !hasItems && !hasIcon,
          'grid-group': hasItems && !hasIcon,
          'grid-icon': !hasItems && hasIcon,
          'grid-full': hasIcon && hasItems,
          nested: hasItems,
        })}
        data-testid={testId}
        onClick={() => {
          if (!disabled && !hasItems) {
            onClick?.();
            onClose?.();
          }
        }}
        {...getReferenceProps()}
      >
        {isPresent(icon) && <Icon icon={icon} size={20} />}
        <p>{text}</p>
        {hasItems && (
          <div className="suffix">
            <Icon icon="arrow-small" size={20} />
          </div>
        )}
      </div>
      {hasItems && submenuOpen && items && (
        <FloatingPortal>
          <Menu
            ref={refs.setFloating}
            style={floatingStyles}
            itemGroups={[{ items }]}
            onClose={onClose}
            {...getFloatingProps()}
          />
        </FloatingPortal>
      )}
    </>
  );
};
