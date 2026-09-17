import { useEffect, useRef, useState } from 'react';
import { isPresent } from '../../../shared/utils/isPresent';

type Props = {
  src: string;
  poster?: string;
};

export const SlideVideo = ({ src, poster }: Props) => {
  // Bundled assets are served without range request support, which makes video
  // playback stutter. Play it from an in-memory blob instead.
  // https://github.com/orgs/tauri-apps/discussions/7870
  const [blobSrc, setBlobSrc] = useState<string>();
  const videoRef = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    let objectUrl: string | undefined;
    let cancelled = false;

    void (async () => {
      const response = await fetch(src);
      const blob = await response.blob();
      if (cancelled) return;
      objectUrl = URL.createObjectURL(blob);
      setBlobSrc(objectUrl);
    })();

    return () => {
      cancelled = true;
      if (isPresent(objectUrl)) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  }, [src]);

  // The welcome window is pre-built hidden and only hidden again on close, so the webview
  // outlives the window being on screen.
  useEffect(() => {
    const video = videoRef.current;
    if (!isPresent(video) || !isPresent(blobSrc)) return;

    const syncPlayback = () => {
      if (document.visibilityState === 'visible') {
        void video.play().catch(() => {
          // Autoplay can be refused while the window is not interactable yet.
        });
      } else {
        video.pause();
      }
    };

    syncPlayback();
    document.addEventListener('visibilitychange', syncPlayback);

    return () => {
      document.removeEventListener('visibilitychange', syncPlayback);
      video.pause();
    };
  }, [blobSrc]);

  return (
    <video
      ref={videoRef}
      src={blobSrc}
      poster={poster}
      width={'100%'}
      height={'auto'}
      loop
      muted
      playsInline
      style={{ overflow: 'hidden' }}
    />
  );
};
