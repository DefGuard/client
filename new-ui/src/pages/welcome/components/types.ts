export interface CarouselSlide {
  title: string;
  slideSrc: string;
  slideType: 'image' | 'video';
  rounded: boolean;
  posterSrc?: string;
  description: string;
  blogLink?: string;
  actionText?: string;
}
