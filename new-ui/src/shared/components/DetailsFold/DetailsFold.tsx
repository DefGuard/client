import './style.scss';

import clsx from 'clsx';
import { Fragment, type ReactNode } from 'react';
import { Divider } from '../Divider/Divider';

export type DetailsFoldRow = {
  label: string;
  value: ReactNode;
};

export type DetailsFoldSection = {
  title: string;
  rows: DetailsFoldRow[];
  compact?: boolean;
};

type Props = {
  sections: DetailsFoldSection[];
};

export const DetailsFold = ({ sections }: Props) => {
  return (
    <div className="details-fold">
      {sections.map((section) => (
        <div className="group" key={section.title}>
          <p>{section.title}</p>
          <div className={clsx('card', { compact: section.compact })}>
            {section.rows.map((row, index) => (
              <Fragment key={row.label}>
                {index > 0 && <Divider />}
                <div className="row">
                  <p>{row.label}</p>
                  <p>{row.value}</p>
                </div>
              </Fragment>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
};
