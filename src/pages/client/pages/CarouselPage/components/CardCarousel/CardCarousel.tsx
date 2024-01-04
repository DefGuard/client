import './style.scss';

import classNames from 'classnames';
import { AnimatePresence, motion } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { HTMLProps, useMemo, useState } from 'react';

import { CarouselControls } from './components/CarouselControl/CarouselControl';
import { CarouselItem } from './types';

type Props = {
  cards: CarouselItem[];
  activeCardIndex?: number;
  onChange?: (index: number) => void;
} & HTMLProps<HTMLDivElement>;

export const CardCarousel = ({
  className,
  cards,
  activeCardIndex,
  onChange,
  ...rest
}: Props) => {
  const [internalIndex, setInternalIndex] = useState(0);

  const cardsCount = useMemo(() => cards.length, [cards.length]);

  const activeIndex = useMemo((): number => {
    if (isUndefined(activeCardIndex)) {
      return internalIndex;
    }
    return activeCardIndex;
  }, [internalIndex, activeCardIndex]);

  const activeCardItem = useMemo((): CarouselItem | undefined => {
    return cards[activeIndex];
  }, [activeIndex, cards]);

  return (
    <div className={classNames('card-carousel', className)} {...rest}>
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.div
          className="card-wrapper"
          key={activeCardItem?.key}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.25 }}
        >
          {activeCardItem?.element}
        </motion.div>
      </AnimatePresence>
      <CarouselControls
        itemsCount={cardsCount}
        currentItemIndex={activeIndex}
        onChange={(index: number) => {
          if (isUndefined(activeCardIndex)) {
            setInternalIndex(index);
          }
          onChange?.(index);
        }}
      />
    </div>
  );
};
