import { mfaMethodIcon } from '../../consts';
import type { MfaMethodValue } from '../../rust-api/types';
import { mfaToText, type OpenIdInstance } from '../../utils/mfa';
import { SelectionItem } from '../SelectionItem/SelectionItem';
import type { SelectionItemProps } from '../SelectionItem/types';

type Props = Omit<SelectionItemProps, 'text' | 'icon'> & {
  method: MfaMethodValue;
  instance?: OpenIdInstance;
};

export const FactorSelector = ({ method, instance, ...props }: Props) => {
  return (
    <SelectionItem
      {...props}
      data-method={method}
      text={mfaToText(method, instance)}
      icon={mfaMethodIcon[method]}
    />
  );
};
