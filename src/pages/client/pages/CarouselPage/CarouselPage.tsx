import './style.scss';

import { useEffect } from 'react';

import { useClientFlags } from '../../hooks/useClientFlags';
import {
  InstancesSlide,
  SecuritySlide,
  SupportSlide,
  TwoFaSlide,
  WelcomeCardSlide,
} from './cards/CarouselCards';
import { CardCarousel } from './components/CardCarousel/CardCarousel';
import type { CarouselItem } from './components/CardCarousel/types';

const slides: CarouselItem[] = [
  {
    key: 'welcome',
    element: <WelcomeCardSlide />,
  },
  {
    key: 'twofa',
    element: <TwoFaSlide />,
  },
  {
    element: <SecuritySlide />,
    key: 'security',
  },
  {
    key: 'instances',
    element: <InstancesSlide />,
  },
  {
    key: 'support',
    element: <SupportSlide />,
  },
];

export const CarouselPage = () => {
  const setClientFlags = useClientFlags((state) => state.setValues);

  useEffect(() => {
    setClientFlags({ firstStart: false });
    // eslint-next-line-ignore
  }, [setClientFlags]);

  return (
    <section className="client-page" id="carousel-page">
      <CardCarousel cards={slides} autoSlide />
    </section>
  );
};
