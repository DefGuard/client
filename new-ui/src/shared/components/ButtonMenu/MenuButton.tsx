import {
  autoUpdate,
  FloatingPortal,
  offset,
  type Placement,
  shift,
  size,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import { useState } from 'react';
import { Button } from '../Button/Button';
import type { ButtonProps } from '../Button/types';
import { Menu } from '../Menu/Menu';
import type { MenuItemsGroup } from '../Menu/types';

export const ButtonMenu = ({
  menuItems,
  placement = 'bottom-end',
  ...props
}: Omit<ButtonProps, 'ref'> & {
  menuItems: MenuItemsGroup[];
  placement?: Placement;
}) => {
  const [isOpen, setOpen] = useState(false);
  const { refs, context, floatingStyles } = useFloating({
    placement,
    whileElementsMounted: autoUpdate,
    onOpenChange: setOpen,
    open: isOpen,
    middleware: [
      offset(4),
      shift({ padding: 4 }),
      size({
        apply({ rects, elements, availableHeight }) {
          const refWidth = `${rects.reference.width}px`;
          elements.floating.style.minWidth = refWidth;
          elements.floating.style.maxHeight = `${availableHeight - 10}px`;
        },
      }),
    ],
  });

  const click = useClick(context, {
    toggle: true,
  });

  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    escapeKey: true,
    outsidePress: (event) => !(event.target as HTMLElement).closest('.menu'),
  });

  const { getFloatingProps, getReferenceProps } = useInteractions([click, dismiss]);

  return (
    <>
      <Button ref={refs.setReference} {...props} {...getReferenceProps()} />
      {isOpen && (
        <FloatingPortal>
          <Menu
            itemGroups={menuItems}
            ref={refs.setFloating}
            style={floatingStyles}
            onClose={() => {
              setOpen(false);
            }}
            {...getFloatingProps()}
          />
        </FloatingPortal>
      )}
    </>
  );
};
