import './style.scss';
import { openUrl } from '@tauri-apps/plugin-opener';
import { AnimatePresence, motion } from 'motion/react';
import { Fragment, useState } from 'react';
import { Button } from '../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../shared/components/Button/types';
import { Divider } from '../../../../shared/components/Divider/Divider';
import { SizedBox } from '../../../../shared/components/SizedBox/SizedBox';
import { motionTransitionStandard } from '../../../../shared/consts';
import { ThemeSpacing } from '../../../../shared/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import { SlideImage } from '../SlideImage';
import { SlideVideo } from '../SlideVideo';
import type { CarouselSlide } from '../types';
import { CarouselControls } from './components/CarouselControls/CarouselControls';
import { CarouselIndicators } from './components/CarouselIndicators/CarouselIndicators';
import type { CarouselDirection, WelcomeCarouselProps } from './types';

const slideVariants = {
  enter: (direction: CarouselDirection) => ({
    x: direction > 0 ? '100%' : '-100%',
    opacity: 0,
  }),
  center: {
    x: 0,
    opacity: 1,
  },
  exit: (direction: CarouselDirection) => ({
    x: direction > 0 ? '-100%' : '100%',
    opacity: 0,
  }),
};

const slideTransition = {
  ...motionTransitionStandard,
  duration: 0.32,
};

const renderSlide = (slide: CarouselSlide) => {
  switch (slide.slideType) {
    case 'image':
      return <SlideImage src={slide.slideSrc} />;
    case 'video':
      return <SlideVideo src={slide.slideSrc} poster={slide.posterSrc} />;
  }
};

export const WelcomeCarousel = ({ slides }: WelcomeCarouselProps) => {
  const [[activeIndex, direction], setSlideState] = useState<[number, CarouselDirection]>(
    [0, 1],
  );

  const isFirst = activeIndex === 0;
  const isLast = activeIndex === slides.length - 1;
  const activeSlide = slides[activeIndex];

  const goTo = (index: number) => {
    if (index === activeIndex) return;
    setSlideState([index, index > activeIndex ? 1 : -1]);
  };
  const goPrev = () => {
    if (!isFirst) goTo(activeIndex - 1);
  };
  const goNext = () => {
    if (!isLast) goTo(activeIndex + 1);
  };

  return (
    <div className="welcome-carousel">
      <div className="carousel-viewport">
        <AnimatePresence mode="popLayout" custom={direction} initial={false}>
          <motion.div
            key={activeIndex}
            className="carousel-slide"
            custom={direction}
            variants={slideVariants}
            initial="enter"
            animate="center"
            exit="exit"
            transition={slideTransition}
          >
            {renderSlide(activeSlide)}
            <div className="content">
              <p className="title">{activeSlide.title}</p>
              <SizedBox height={ThemeSpacing.Sm} />
              <p className="description">{activeSlide.description}</p>
              {isPresent(activeSlide.blogLink) && isPresent(activeSlide.blogLinkText) && (
                <Fragment>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <Button
                    text={activeSlide.blogLinkText}
                    variant={ButtonVariant.Primary}
                    onClick={() => openUrl(activeSlide.blogLink as string)}
                  />
                </Fragment>
              )}
            </div>
          </motion.div>
        </AnimatePresence>
      </div>
      {slides.length > 1 && (
        <Fragment>
          <Divider spacing={ThemeSpacing.Xl} />
          <div className="carousel-footer">
            <CarouselControls
              onPrev={goPrev}
              onNext={goNext}
              disablePrev={isFirst}
              disableNext={isLast}
            >
              <CarouselIndicators
                count={slides.length}
                activeIndex={activeIndex}
                onSelect={goTo}
              />
            </CarouselControls>
          </div>
        </Fragment>
      )}
    </div>
  );
};
