import { AnimatePresence } from 'motion/react';
import { type PropsWithChildren, useCallback, useEffect, useRef, useState } from 'react';
import { Fragment } from 'react/jsx-runtime';
import { createPortal } from 'react-dom';
import { useDeferredCallback } from '../../hooks/useDeferredCallback';
import { isPresent } from '../../utils/isPresent';
import { SnackbarElement } from './SnackbarElement/SnackbarElement';
import { type SnackbarConfig, SnackbarVariant } from './types';
import { useSnackbarStore } from './useSnackbarStore';

const portalTarget = document.getElementById('snackbar-root') as HTMLElement;

// if something goes wrong or user ignores dismissible snackbar we will close it anyway
const fallbackTimeout = 90_000;

const regularTimeout = 5_000;

const RenderPortal = ({ children }: PropsWithChildren) => {
  return createPortal(children, portalTarget);
};

export const SnackbarManager = ({ children }: PropsWithChildren) => {
  const [visible, setVisible] = useState(false);
  const [activeSnackbar, setActiveSnackbar] = useState<SnackbarConfig | null>(null);
  // next in line, this should be always latest call from subject, anything in between should be lost
  const nextSnackRef = useRef<SnackbarConfig | null>(null);
  // if active is busy, so we won't need to update the effect
  const activeBusyRef = useRef(false);

  const closeActiveSnackbar = useCallback(() => {
    setVisible(false);
  }, []);

  const { start: startAutoCloseSnackbar, cancel: cancelAutoClose } =
    useDeferredCallback(closeActiveSnackbar);

  const setupAutoClose = useCallback(
    (config: SnackbarConfig) => {
      if (config.variant === SnackbarVariant.Loading || config.dismissible) {
        startAutoCloseSnackbar(fallbackTimeout);
      } else {
        startAutoCloseSnackbar(regularTimeout);
      }
    },
    [startAutoCloseSnackbar],
  );

  const handleSnackbarDismiss = useCallback(() => {
    cancelAutoClose();
    closeActiveSnackbar();
  }, [cancelAutoClose, closeActiveSnackbar]);

  const popActiveSnackbar = useCallback(() => {
    if (!activeSnackbar) return;
    if (nextSnackRef.current) {
      const snackbarConfig = { ...nextSnackRef.current };
      setVisible(true);
      setupAutoClose(snackbarConfig);
      setActiveSnackbar(snackbarConfig);
      nextSnackRef.current = null;
    } else {
      // no next snackbar
      setActiveSnackbar(null);
      activeBusyRef.current = false;
    }
  }, [activeSnackbar, setupAutoClose]);

  // process incoming requests for snackbar's
  useEffect(() => {
    const sub = useSnackbarStore.getState().snackSubject.subscribe((snackbar) => {
      if (activeBusyRef.current) {
        nextSnackRef.current = snackbar;
      } else {
        // there isn't any active snackbar
        setActiveSnackbar(snackbar);
        setupAutoClose(snackbar);
        setVisible(true);
        activeBusyRef.current = true;
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [setupAutoClose]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: updates active snackbar's
  useEffect(() => {
    const sub = useSnackbarStore.getState().updateSubject.subscribe((updateEvent) => {
      setActiveSnackbar((currentState) => {
        // ignore invalid updates
        if (!currentState) return null;
        if (!currentState.id) return currentState;
        // ignore if update was meant for another snackbar then the current one
        if (currentState.id !== updateEvent.id) return currentState;
        const newState = { ...currentState, ...updateEvent.update };
        if (updateEvent.resetAutoDismiss) {
          setupAutoClose(newState);
        }
        return newState;
      });
    });
    return () => {
      sub.unsubscribe();
    };
  }, [setActiveSnackbar]);

  useEffect(() => {
    const sub = useSnackbarStore.getState().closeSubject.subscribe((closeTarget) => {
      if (activeSnackbar?.id && activeSnackbar.id === closeTarget) {
        handleSnackbarDismiss();
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [activeSnackbar, handleSnackbarDismiss]);

  useEffect(() => {
    const sub = useSnackbarStore.getState().clearSubject.subscribe(() => {
      nextSnackRef.current = null;
      activeBusyRef.current = false;
      cancelAutoClose();
      setVisible(false);
      setActiveSnackbar(null);
    });
    return () => {
      sub.unsubscribe();
    };
  }, [cancelAutoClose]);

  return (
    <Fragment>
      {children}
      <RenderPortal>
        <AnimatePresence mode="wait">
          {isPresent(activeSnackbar) && visible && (
            <SnackbarElement
              data={activeSnackbar}
              onClose={handleSnackbarDismiss}
              onExitAnimationEnd={popActiveSnackbar}
            />
          )}
        </AnimatePresence>
      </RenderPortal>
    </Fragment>
  );
};
