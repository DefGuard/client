import { useEffect, useRef } from 'react';

export const MainBackground = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const handleResize = () => {
      // Get parent dimensions
      // biome-ignore lint/style/noNonNullAssertion: Always have parent
      const { clientWidth: w, clientHeight: h } = canvas.parentElement!;

      // Update internal resolution
      canvas.width = w;
      canvas.height = h;

      // Draw Gradient (134deg)
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

    // Initial draw
    handleResize();

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);
  return (
    <canvas
      ref={canvasRef}
      style={{
        display: 'block',
        position: 'fixed',
        inset: 0,
        width: '100%',
        height: '100%',
        zIndex: -1,
      }}
    />
  );
};
