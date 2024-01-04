import './style.scss';

import classNames from 'classnames';
import { range } from 'lodash-es';

type Props = {
  itemsCount: number;
  currentItemIndex: number;
  onChange: (index: number) => void;
};

export const CarouselControls = ({ itemsCount, currentItemIndex, onChange }: Props) => {
  if (itemsCount < 0) return null;

  return (
    <div className="carousel-control">
      {range(itemsCount).map((index) => (
        <button
          key={index}
          data-testid={`control-dot-${index}`}
          onClick={() => {
            onChange?.(index);
          }}
        >
          <span
            className={classNames('dot', {
              active: index === currentItemIndex,
            })}
          ></span>
        </button>
      ))}
    </div>
  );
};
