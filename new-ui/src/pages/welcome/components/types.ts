export interface CarouselSlide {
  title: string;
  slideSrc: string;
  slideType: 'image' | 'video';
  posterSrc?: string;
  description: string;
  blogLink?: string;
  blogLinkText?: string;
}
