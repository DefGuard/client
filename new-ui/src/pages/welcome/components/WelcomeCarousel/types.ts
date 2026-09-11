import type { CarouselSlide } from '../types';

export interface WelcomeCarouselProps {
  slides: CarouselSlide[];
}

// +1 = advancing forward (next), -1 = going back (prev) — drives slide-in/out direction
export type CarouselDirection = 1 | -1;
