import type { ModalBase } from '../ModalFoundation/types';

export interface ModalProps extends ModalBase {
  title: string;
  size?: 'small' | 'primary';
}
