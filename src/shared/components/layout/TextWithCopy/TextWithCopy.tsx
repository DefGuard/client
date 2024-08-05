import { clipboard } from '@tauri-apps/api';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { TextContainer } from '../../../defguard-ui/components/Layout/TextContainer/TextContainer';
import { useToaster } from '../../../defguard-ui/hooks/toasts/useToaster';

type Props = {
  text: string;
};

/**Wrapper Around @link TextContainer */
export const TextWithCopy = ({ text }: Props) => {
  const toaster = useToaster();
  const { LL } = useI18nContext();
  return (
    <TextContainer
      text={text}
      onClick={(text: string) => {
        clipboard.writeText(text).then(() => {
          toaster.success(LL.common.messages.clipboardGeneric());
        });
      }}
    />
  );
};
