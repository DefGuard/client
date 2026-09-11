import { useEffect, useRef } from 'react';

export const ModalGradient = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const draw = (w: number, h: number) => {
      canvas.width = w;
      canvas.height = h;

      const angle = (134 * Math.PI) / 180;
      const length = Math.sqrt(w ** 2 + h ** 2);

      const x1 = w / 2 - (Math.sin(angle) * length) / 2;
      const y1 = h / 2 + (Math.cos(angle) * length) / 2;
      const x2 = w / 2 + (Math.sin(angle) * length) / 2;
      const y2 = h / 2 - (Math.cos(angle) * length) / 2;

      const gradient = ctx.createLinearGradient(x1, y1, x2, y2);
      gradient.addColorStop(0, '#5B83FF');
      gradient.addColorStop(1, '#0036DB');

      ctx.fillStyle = gradient;
      ctx.fillRect(0, 0, w, h);
    };

    // biome-ignore lint/style/noNonNullAssertion: Always have parent
    const parent = canvas.parentElement!;
    draw(parent.clientWidth, parent.clientHeight);

    const observer = new ResizeObserver(([entry]) => {
      const { inlineSize: w, blockSize: h } = entry.contentBoxSize[0];
      draw(w, h);
    });

    observer.observe(parent);
    return () => observer.disconnect();
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{
        display: 'block',
        position: 'absolute',
        inset: 0,
        width: '100%',
        height: '100%',
        zIndex: -1,
      }}
    />
  );
};
