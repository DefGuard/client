import './style.scss';

import classNames from 'classnames';
import { AnimatePresence, motion } from 'framer-motion';
import { isUndefined } from 'lodash-es';
import { type HTMLProps, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { interval } from 'rxjs';

import { CarouselControls } from './components/CarouselControl/CarouselControl';
import type { CarouselItem } from './types';

type Props = {
  cards: CarouselItem[];
  activeCardIndex?: number;
  onChange?: (index: number) => void;
  /** Progress slides if main container is not hovered, this can only be used when activeCardIndex is NOT provided */
  autoSlide?: boolean;
  /** How often carousel will change, in milisenconds*/
  autoSlideInterval?: number;
} & HTMLProps<HTMLDivElement>;

export const CardCarousel = ({
  className,
  cards,
  activeCardIndex,
  onChange,
  autoSlide = false,
  autoSlideInterval = 4000,
  ...rest
}: Props) => {
  const hoveredRef = useRef<boolean>(false);
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

  const nextSlide = useCallback(() => {
    setInternalIndex((currentIndex) => {
      if (currentIndex === cardsCount - 1) {
        return 0;
      }
      return currentIndex + 1;
    });
  }, [cardsCount]);

  useEffect(() => {
    if (autoSlide) {
      const sub = interval(autoSlideInterval).subscribe(() => {
        if (!hoveredRef.current) {
          nextSlide();
        }
      });
      return () => {
        sub.unsubscribe();
      };
    }
  }, [nextSlide, autoSlide, autoSlideInterval]);

  return (
    <div
      className={classNames('card-carousel', className)}
      onMouseEnter={() => {
        hoveredRef.current = true;
      }}
      onMouseLeave={() => {
        hoveredRef.current = false;
      }}
      {...rest}
    >
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
