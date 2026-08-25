import whatsNewVideo from './assets/whats_new.mp4';
import whatsNewPoster from './assets/whats_new_poster.jpg?inline';
import type { CarouselSlide } from './components/types';

export const welcomeSlides: CarouselSlide[] = [
  {
    slideSrc: whatsNewVideo,
    slideType: 'video',
    posterSrc: whatsNewPoster,
    title: `New Defguard Desktop App`,
    description: `Defguard now lives in your system tray, giving you faster access while staying out of your way. Launch it anytime directly from the tray.`,
    blogLink: 'https://defguard.net/blog/',
    blogLinkText: 'Check how it works',
  },
];
