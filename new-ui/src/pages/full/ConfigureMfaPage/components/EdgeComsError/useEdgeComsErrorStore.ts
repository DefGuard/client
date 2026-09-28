import { create } from 'zustand';

type Store = {
  visible: boolean;
  retry: (() => void) | null;
};

export const useEdgeComsErrorStore = create<Store>(() => ({
  visible: false,
  retry: null,
}));

/** For when a request could not reach the proxy at all, as opposed to the proxy rejecting it. */
export const showEdgeComsError = (retry?: () => void): void => {
  useEdgeComsErrorStore.setState({ visible: true, retry: retry ?? null });
};

export const dismissEdgeComsError = (): void => {
  useEdgeComsErrorStore.setState({ visible: false, retry: null });
};

export const retryEdgeComsError = (): void => {
  const { retry } = useEdgeComsErrorStore.getState();
  dismissEdgeComsError();
  retry?.();
};
