import slide1 from './assets/slide-1.png';
import slide2 from './assets/slide-2.png';
import slide3 from './assets/slide-3.png';
import slide4 from './assets/slide-4.png';
import type { CarouselSlide } from './components/types';

export const welcomeSlides: CarouselSlide[] = [
  {
    slideSrc: slide1,
    slideType: 'image',
    rounded: false,
    title: `Multi-step MFA`,
    description: `Your admin may require several verification steps to connect to a VPN location, for example Google sign-in followed by your authenticator app.`,
    actionText: 'Get Started',
  },
  {
    slideSrc: slide2,
    slideType: 'image',
    rounded: false,
    title: `FIDO2 security keys`,
    description: `Register a FIDO2 security key in the app and use it as a secure, phishing-resistant MFA method for authentication.`,
    actionText: 'Get Started',
  },
  {
    slideSrc: slide3,
    slideType: 'image',
    rounded: false,
    title: `Add new MFA methods anytime`,
    description: `Manage, add, and configure your MFA methods whenever you want, directly in the desktop app, with just a few clicks.`,
    actionText: 'Get Started',
  },
  {
    slideSrc: slide4,
    slideType: 'image',
    rounded: false,
    title: `Configure missing MFA in the app`,
    description: `If a location requires an MFA method you haven't set up, the Connect button changes to Configure MFA so you can set it up right away.`,
    actionText: 'Get Started',
  },
];
