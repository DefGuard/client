import './style.scss';

import {
  InstancesSlide,
  SecuritySlide,
  SupportSlide,
  TwoFaSlide,
  WelcomeCardSlide,
} from './cards/CarouselCards';
import { CardCarousel } from './components/CardCarousel/CardCarousel';
import { CarouselItem } from './components/CardCarousel/types';

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
  return (
    <section className="client-page" id="carousel-page">
      <CardCarousel cards={slides} />
    </section>
  );
};
