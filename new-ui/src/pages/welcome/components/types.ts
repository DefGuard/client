export interface CarouselSlide {
  title: string;
  slideSrc: string;
  slideType: 'image' | 'video';
  description: string;
  blogLink?: string;
  blogLinkText?: string;
}
