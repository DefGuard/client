import './style.scss';
import type { TabsProps } from './types';

export const Tabs = ({ items }: TabsProps) => {
  return (
    <div className="tabs" role="tablist">
      {items.map((item) => (
        <button
          key={item.title}
          type="button"
          className="tab"
          role="tab"
          aria-selected={item.active}
          data-active={item.active}
          onClick={item.onClick}
        >
          <span>{item.title}</span>
          <span className="line"></span>
        </button>
      ))}
    </div>
  );
};
