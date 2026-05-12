import './style.scss';
import {
  autoUpdate,
  FloatingPortal,
  flip,
  size as floatingSize,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import clsx from 'clsx';
import { Fragment, type JSX, useCallback, useId, useMemo, useState } from 'react';
import { Direction, ThemeSpacing, ThemeVariable } from '../../types';
import { isPresent } from '../../utils/isPresent';
import { Divider } from '../Divider/Divider';
import { FieldBox } from '../FieldBox/FieldBox';
import { FieldError } from '../FieldError/FieldError';
import { FieldLabel } from '../FieldLabel/FieldLabel';
import { FloatingMenu } from '../FloatingMenu/FloatingMenu';
import { Icon, IconKind } from '../Icon';
import type { SelectOption, SelectOptionGroup, SelectProps } from './types';

export function Select<T>(props: SelectProps<T>): JSX.Element {
  const labelId = useId();

  const {
    label,
    options,
    groups,
    className,
    placeholder,
    testId,
    error,
    size = 'default',
    disabled = false,
    required = false,
  } = props;

  const [floatingOpen, setFloatingOpen] = useState(false);

  const { refs, context, floatingStyles } = useFloating({
    placement: 'bottom-start',
    open: floatingOpen,
    onOpenChange: setFloatingOpen,
    middleware: [
      offset(4),
      flip(),
      shift(),
      floatingSize({
        apply({ rects, elements, availableHeight }) {
          const refWidth = `${rects.reference.width}px`;
          elements.floating.style.minWidth = refWidth;
          elements.floating.style.maxHeight = `${availableHeight - 10}px`;
        },
      }),
    ],
    whileElementsMounted: autoUpdate,
  });

  const selectedLabel = useMemo(() => props.value?.label ?? null, [props.value]);

  const renderedGroups: readonly SelectOptionGroup<T>[] = groups ?? [];
  const renderedOptions: readonly SelectOption<T>[] = options ?? [];

  // biome-ignore lint/correctness/useExhaustiveDependencies: onChange
  const handleChange = useCallback(
    (option: SelectOption<T>, isSelected: boolean) => {
      if (isSelected) return;
      props.onChange(option);
      setFloatingOpen(false);
    },
    [props.onChange, setFloatingOpen],
  );

  const click = useClick(context, {
    toggle: true,
    enabled: !disabled,
  });

  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    escapeKey: true,
    outsidePress: true,
  });

  const { getFloatingProps, getReferenceProps } = useInteractions([click, dismiss]);

  return (
    <>
      <div className="select spacer">
        <div
          className={clsx('inner', {
            disabled,
          })}
        >
          {isPresent(label) && (
            <FieldLabel required={required} id={labelId} text={label} />
          )}
          <FieldBox
            className={className}
            disabled={disabled}
            error={!disabled && isPresent(error)}
            size={size}
            boxRef={refs.setReference}
            data-testid={testId}
            iconRight={
              <Icon
                icon={IconKind.ArrowSmall}
                size={20}
                staticColor={ThemeVariable.FgWhite100}
                rotationDirection={floatingOpen ? Direction.DOWN : Direction.RIGHT}
              />
            }
            forceFocusState={floatingOpen}
            aria-labelledby={labelId}
            {...getReferenceProps()}
          >
            <div className="box-track">
              {isPresent(placeholder) && !isPresent(selectedLabel) && (
                <span className="placeholder">{placeholder}</span>
              )}
              {isPresent(selectedLabel) && <span className="value">{selectedLabel}</span>}
            </div>
          </FieldBox>
          <FieldError error={error} />
        </div>
      </div>
      {floatingOpen && (
        <FloatingPortal>
          <FloatingMenu
            containerProps={{
              role: 'list',
              className: 'select-floating',
              ref: refs.setFloating,
              style: { position: 'absolute', ...floatingStyles },
              ...getFloatingProps(),
            }}
          >
            {renderedOptions.map((option, optionIndex) => {
              const isSelected = props.value?.key === option.key;
              const isLast = renderedOptions.length - 1 === optionIndex;
              return (
                <SelectOptionItem
                  isLast={isLast}
                  isSelected={isSelected}
                  key={`option-${optionIndex}-${option.key}`}
                  onSelect={handleChange}
                  option={option}
                />
              );
            })}
            {renderedGroups.map((group, groupIndex) => {
              const isLast = renderedGroups.length - 1 === groupIndex;
              const groupKey = group.key ?? `${group.label}-${groupIndex}`;

              return (
                <Fragment key={`group-${groupKey}`}>
                  <div className="select-group" role="group" aria-label={group.label}>
                    <div className="section-title" role="presentation">
                      <p>{group.label}</p>
                    </div>
                  </div>
                  {group.options.map((option, optionIndex) => {
                    const isSelected = props.value?.key === option.key;
                    const isLast = group.options.length - 1 === optionIndex;
                    return (
                      <SelectOptionItem
                        isLast={isLast}
                        isSelected={isSelected}
                        key={`group-option-${groupKey}-${optionIndex}-${option.key}`}
                        onSelect={handleChange}
                        option={option}
                      />
                    );
                  })}
                  {!isLast && <Divider spacing={ThemeSpacing.Sm} />}
                </Fragment>
              );
            })}
          </FloatingMenu>
        </FloatingPortal>
      )}
    </>
  );
}

type SelectOptionItemProps<T> = {
  isLast: boolean;
  isSelected: boolean;
  onSelect: (option: SelectOption<T>, isSelected: boolean) => void;
  option: SelectOption<T>;
};

function SelectOptionItem<T>({
  isLast,
  isSelected,
  onSelect,
  option,
}: SelectOptionItemProps<T>): JSX.Element {
  return (
    <div
      className={clsx('select-option', {
        selected: isSelected,
        last: isLast,
      })}
      onClick={() => {
        onSelect(option, isSelected);
      }}
      role="listitem"
    >
      <span>{option.label}</span>
      {isSelected && (
        <Icon icon="check" staticColor={ThemeVariable.FgWhite100} size={20} />
      )}
    </div>
  );
}
