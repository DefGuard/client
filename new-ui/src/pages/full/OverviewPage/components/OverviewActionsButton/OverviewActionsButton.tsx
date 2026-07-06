import {
  autoUpdate,
  FloatingPortal,
  offset,
  shift,
  size,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import { Icon } from '../../../../../shared/components/Icon';
import { ThemeVariable } from '../../../../../shared/types';
import './style.scss';
import { error } from '@tauri-apps/plugin-log';
import { useCallback, useMemo, useState } from 'react';
import { Menu } from '../../../../../shared/components/Menu/Menu';
import type { MenuItemsGroup } from '../../../../../shared/components/Menu/types';
import { openModal } from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import { api } from '../../../../../shared/rust-api/api';
import type {
  InstanceInfo,
  OverviewViewSelection,
} from '../../../../../shared/rust-api/types';

type Props = {
  selection: OverviewViewSelection | null;
  instance: InstanceInfo | null;
};

export const OverviewActionsButton = ({ selection, instance }: Props) => {
  const [isOpen, setOpen] = useState(false);

  const { refs, context, floatingStyles } = useFloating({
    placement: 'bottom-end',
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

  const handleUpdate = useCallback(async () => {
    if (!selection) return;
    try {
      switch (selection.kind) {
        case 'tunnel': {
          const tunnel = await api.getTunnelDetails(selection.id);
          openModal(ModalName.UpdateTunnel, tunnel);
          break;
        }
        case 'instance':
          if (!instance) break;
          openModal(ModalName.UpdateInstance, {
            instanceId: instance.id,
            url: instance.proxy_url,
          });
          break;
      }
    } catch (e) {
      error(`Failed update action, ${String(e)}`);
    }
  }, [selection, instance]);

  const handleDelete = useCallback(async () => {
    if (!selection) return;
    try {
      switch (selection.kind) {
        case 'instance':
          await api.deleteInstance(selection.id);
          break;
        case 'tunnel':
          await api.deleteTunnel(selection.id);
          break;
      }
    } catch (e) {
      error(`Failed delete action, ${String(e)}`);
    }
  }, [selection]);

  const menuConfig = useMemo(() => {
    const config: MenuItemsGroup[] = [
      {
        items: [
          {
            text: selection?.kind === 'instance' ? 'Update' : 'Edit',
            icon: 'refresh',
            onClick: handleUpdate,
          },
          {
            text: 'Delete',
            icon: 'delete',
            onClick: handleDelete,
          },
        ],
      },
    ];
    return config;
  }, [handleUpdate, handleDelete, selection?.kind]);

  const text = () => {
    switch (selection?.kind) {
      case 'instance':
        return `Instance settings`;
      case 'tunnel':
        return `Tunnel settings`;
    }
  };

  return (
    <>
      <button
        className="overview-header-actions"
        ref={refs.setReference}
        {...getReferenceProps()}
      >
        <p>{text()}</p>
        <Icon
          icon="arrow-small"
          rotationDirection="down"
          staticColor={ThemeVariable.FgWhite80}
          size={16}
        />
      </button>
      {isOpen && (
        <FloatingPortal>
          <Menu
            itemGroups={menuConfig}
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
