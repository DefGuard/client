import type {
  HTMLAttributes,
  HTMLInputAutoCompleteAttribute,
  MouseEventHandler,
  Ref,
} from 'react';
import type { FieldBoxProps, FieldSize } from '../FieldBox/types';

export type InputProps = {
  value: string | null | number;
  size?: FieldSize;
  type?: 'password' | 'text' | 'search' | 'number';
  ref?: Ref<HTMLInputElement>;
  error?: string | null;
  name?: string;
  label?: string;
  required?: boolean;
  disabled?: boolean;
  placeholder?: string;
  onChange?: (value: string | number | null) => void;
  boxProps?: Partial<FieldBoxProps>;
  autocomplete?: HTMLInputAutoCompleteAttribute;
  testId?: string;
  notNull?: boolean;
} & Pick<HTMLAttributes<HTMLInputElement>, 'onBlur' | 'onFocus'>;

export type FormInputProps = Pick<
  InputProps,
  | 'name'
  | 'placeholder'
  | 'disabled'
  | 'required'
  | 'label'
  | 'autocomplete'
  | 'size'
  | 'type'
  | 'notNull'
> & {
  mapError?: (error: string) => string | undefined;
  onDismiss?: MouseEventHandler<HTMLButtonElement>;
};
