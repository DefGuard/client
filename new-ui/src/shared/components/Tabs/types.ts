export type TabsItem = {
  title: string;
  active: boolean;
  onClick: () => void;
};

export type TabsProps = {
  items: TabsItem[];
};
