import { create } from 'zustand';

type Store = {
  visible: boolean;
};

export const useEdgeComsErrorStore = create<Store>(() => ({ visible: false }));

/** For when a request could not reach the proxy at all, as opposed to the proxy rejecting it. */
export const showEdgeComsError = (): void => {
  useEdgeComsErrorStore.setState({ visible: true });
};

export const dismissEdgeComsError = (): void => {
  useEdgeComsErrorStore.setState({ visible: false });
};
