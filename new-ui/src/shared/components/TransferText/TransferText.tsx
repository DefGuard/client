import byteSize from 'byte-size';
import clsx from 'clsx';
import './style.scss';

type Props = {
  data: number;
};

export const TransferText = ({ data }: Props) => {
  const size = byteSize(data, { precision: 1 });

  return (
    <div className={clsx('transfer-text')}>
      <span>{`${size.value} ${size.unit}`}</span>
    </div>
  );
};
