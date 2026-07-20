import './style.scss';
import clsx from 'clsx';

type Props = {
  count: number;
  activeIndex: number;
  onSelect: (index: number) => void;
};

export const CarouselIndicators = ({ count, activeIndex, onSelect }: Props) => {
  return (
    <div className="carousel-indicators">
      {Array.from({ length: count }, (_, index) => (
        <button
          key={index}
          type="button"
          className={clsx('indicator', { active: index === activeIndex })}
          aria-current={index === activeIndex}
          aria-label={`Go to slide ${index + 1}`}
          onClick={() => onSelect(index)}
        >
          <span className="dot"></span>
        </button>
      ))}
    </div>
  );
};
