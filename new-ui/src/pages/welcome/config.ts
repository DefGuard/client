import whatsNewVideo from './assets/whats_new.mp4';
import whatsNewPoster from './assets/whats_new_poster.jpg?inline';
import type { CarouselSlide } from './components/types';

export const welcomeSlides: CarouselSlide[] = [
  {
    slideSrc: whatsNewVideo,
    slideType: 'video',
    posterSrc: whatsNewPoster,
    title: `New Defguard Desktop App`,
    description: `A new compact interface gives you one-click connections and quick switching between locations, while the redesigned main window puts settings and logs within instant reach - all in a cleaner, more intuitive design.`,
    blogLink: 'https://defguard.net/blog/',
    blogLinkText: 'Check how it works',
  },
];
