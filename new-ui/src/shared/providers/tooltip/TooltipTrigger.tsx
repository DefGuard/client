/** biome-ignore-all lint/suspicious/noExplicitAny: needs to be like this */
import { useMergeRefs } from '@floating-ui/react';
import { cloneElement, type HTMLProps, isValidElement, type Ref } from 'react';
import { useTooltipContext } from './TooltipContext';

type Props = {
  ref?: Ref<HTMLElement>;
} & HTMLProps<HTMLElement>;

export const TooltipTrigger = ({ children, ref: propRef, ...props }: Props) => {
  const context = useTooltipContext();
  const childrenRef = (children as any).ref;
  const ref = useMergeRefs([context.refs.setReference, propRef, childrenRef]);

  if (!isValidElement(children))
    throw new Error('Tooltip Trigger child is not an valid react element!');

  return cloneElement(
    children,
    context.getReferenceProps({
      ref,
      ...props,
      ...(children as any).props,
      'data-state': context.open ? 'open' : 'closed',
    }),
  );
};
