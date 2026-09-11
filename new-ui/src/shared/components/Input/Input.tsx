import { type HTMLInputTypeAttribute, useId, useMemo, useRef, useState } from 'react';
import './style.scss';
import clsx from 'clsx';
import { isNumber } from 'radashi';
import { isPresent } from '../../utils/isPresent';
import { mergeRefs } from '../../utils/mergeRefs';
import { FieldBox } from '../FieldBox/FieldBox';
import { FieldError } from '../FieldError/FieldError';
import { FieldLabel } from '../FieldLabel/FieldLabel';
import { Icon } from '../Icon';
import type { IconKindValue } from '../Icon/icon-types';
import type { InputProps } from './types';

const externalValueToInput = (value: string | null | number): string | number => {
  if (value === null) return '';
  return value;
};

const preferredTypeToInternal = (value: InputProps['type']): HTMLInputTypeAttribute => {
  if (!isPresent(value)) return 'text';

  switch (value) {
    case 'search':
      return 'text';
    case 'number':
      return 'number';
    default:
      return value;
  }
};

export const Input = ({
  value,
  error,
  label,
  ref,
  name,
  placeholder,
  boxProps,
  testId,
  onChange,
  onBlur,
  onFocus,
  notNull,
  size = 'default',
  type = 'text',
  required = false,
  disabled = false,
  autocomplete = 'off',
}: InputProps) => {
  const isPassword = useMemo(() => type === 'password', [type]);

  const preferredTypeIsSearch = useMemo(() => type === 'search', [type]);

  const [inputTypeInner, setInputType] = useState<HTMLInputTypeAttribute>(
    preferredTypeToInternal(type),
  );

  const innerRef = useRef<HTMLInputElement>(null);
  const id = useId();

  const interactionIconRight = useMemo((): IconKindValue | undefined => {
    if (typeof value === 'string') {
      // allow clear action for search
      if (value?.length && preferredTypeIsSearch) {
        return 'clear';
      }
      // toggle show / hide for password
      if (isPassword) {
        if (inputTypeInner === 'password') {
          return 'show';
        } else {
          return 'hide';
        }
      }
    }
  }, [isPassword, inputTypeInner, value, preferredTypeIsSearch]);

  return (
    <div className="input spacer">
      <div
        className={clsx('inner', {
          disabled,
        })}
      >
        {isPresent(label) && (
          <FieldLabel
            id={id}
            required={required}
            text={label}
            onClick={() => {
              innerRef.current?.focus();
            }}
          />
        )}
        <FieldBox
          className="input-track"
          error={!disabled && isPresent(error)}
          disabled={disabled}
          size={size}
          onClick={() => {
            innerRef.current?.focus();
          }}
          iconLeft={preferredTypeIsSearch ? <Icon icon="search" /> : undefined}
          iconRight={
            interactionIconRight ? <Icon icon={interactionIconRight} /> : undefined
          }
          reserveInteraction={preferredTypeIsSearch}
          onInteractionClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            // clear
            if (preferredTypeIsSearch) {
              onChange?.('');
            }
            if (isPassword) {
              setInputType((s) => {
                if (s === 'password') {
                  return 'text';
                }
                return 'password';
              });
            }
          }}
          {...boxProps}
        >
          <input
            aria-labelledby={id}
            ref={mergeRefs([ref, innerRef])}
            autoComplete={autocomplete}
            data-testid={testId}
            value={externalValueToInput(value)}
            name={name}
            type={inputTypeInner}
            placeholder={placeholder}
            disabled={disabled}
            onFocus={onFocus}
            onBlur={onBlur}
            onChange={(e) => {
              if (isPresent(onChange)) {
                let changeValue: string | null | number = e.target.value;
                // allows nulls to be typed directly to form state
                if (changeValue === '' && !notNull && !required) {
                  changeValue = null;
                } else {
                  if (inputTypeInner === 'number') {
                    const parsed = parseInt(changeValue, 10);
                    if (!isNumber(parsed)) return;
                    changeValue = parsed;
                  }
                }
                onChange(changeValue);
              }
            }}
          />
        </FieldBox>
        <FieldError error={disabled ? undefined : error} />
      </div>
    </div>
  );
};
