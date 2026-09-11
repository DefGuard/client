import type { FieldBoxProps } from '../FieldBox/types';

export type SelectOption<T> = {
  key: string | number;
  label: string;
  value: T;
  meta?: unknown;
};

export type SelectOptionGroup<T> = {
  key?: string | number;
  label: string;
  options: readonly SelectOption<T>[];
};

export type SelectSingleValue<T> = SelectOption<T>;

type SelectOptionsSourceProps<T> =
  | {
      options: readonly SelectOption<T>[];
      groups?: never;
    }
  | {
      options?: never;
      groups: readonly SelectOptionGroup<T>[];
    }
  | {
      options: readonly SelectOption<T>[];
      groups: readonly SelectOptionGroup<T>[];
    };

type BaseProps<T> = {
  testId?: string;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  label?: string;
  required?: boolean;
  error?: string;
} & Pick<FieldBoxProps, 'size'> &
  SelectOptionsSourceProps<T>;

export type SelectSingleProps<T> = BaseProps<T> & {
  value: SelectSingleValue<T>;
  onChange: (v: SelectSingleValue<T>) => void;
};

export type SelectProps<T> = SelectSingleProps<T>;
