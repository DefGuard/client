import testSlide from './assets/test_frame.png';
import type { CarouselSlide } from './components/types';

export const welcomeSlides: CarouselSlide[] = [
  {
    slideSrc: testSlide,
    slideType: 'image',
    title: `New Defguard Desktop App`,
    description: `Defguard now lives in your system tray, giving you faster access while staying out of your way. Launch it anytime directly from the tray.`,
    blogLink: 'https://defguard.net/blog/',
    blogLinkText: 'Check how it works',
  },
  {
    slideSrc: testSlide,
    slideType: 'image',
    title: `Faster Connection Switching`,
    description: `Switch between locations in a single click, right from the tray menu, without opening the full app window.`,
    blogLink: 'https://defguard.net/blog/',
    blogLinkText: 'See what changed',
  },
  {
    slideSrc: testSlide,
    slideType: 'image',
    title: `Improved Security Posture Checks`,
    description: `Posture checks now run silently in the background and surface only actionable issues, reducing noise while keeping you compliant.`,
  },
  {
    slideSrc: testSlide,
    slideType: 'image',
    title: `Redesigned Notifications`,
    description: `Native system notifications now match your OS theme and give you quick actions without switching windows.`,
    blogLink: 'https://defguard.net/blog/',
    blogLinkText: 'Read the release notes',
  },
];
