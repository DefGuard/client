import type { PropsWithChildren } from 'react';

export interface ModalBase extends PropsWithChildren {
  isOpen: boolean;
  id?: string;
  hideBackdrop?: boolean;
  positionerClassName?: string;
  contentClassName?: string;
  onClose?: (() => void) | null;
  afterClose?: () => void;
}
