import './style.scss';

import classNames from 'classnames';
import { HtmlHTMLAttributes, ReactNode, useMemo } from 'react';

type Props = HtmlHTMLAttributes<HTMLDivElement> & {
  children?: ReactNode;
};

export const PageContainer = ({ children, className, ...rest }: Props) => {
  const cn = useMemo(() => classNames('page-container', className), [className]);
  return (
    <div className={cn} {...rest}>
      {children}
    </div>
  );
};
