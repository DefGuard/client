import type {
  HTMLAttributes,
  MouseEventHandler,
  PropsWithChildren,
  ReactNode,
  Ref,
} from 'react';

export type FieldSize = 'lg' | 'default';

export interface FieldBoxProps extends HTMLAttributes<HTMLDivElement>, PropsWithChildren {
  boxRef?: Ref<HTMLDivElement>;
  interactionRef?: Ref<HTMLDivElement>;
  error?: boolean;
  disabled?: boolean;
  iconLeft?: ReactNode;
  iconRight?: ReactNode;
  size?: FieldSize;
  forceFocusState?: boolean;
  onInteractionClick?: MouseEventHandler<HTMLButtonElement>;
  reserveInteraction?: boolean;
}
