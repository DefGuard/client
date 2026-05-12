export const downloadText = (
  content: string,
  filename: string,
  extension: 'txt' | 'pub' | 'conf' = 'txt',
) => {
  const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
  downloadFile(blob, filename, extension);
};

export const downloadFile = (blob: Blob, filename: string, extension: string) => {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');

  link.href = url;
  link.style = 'visibility: hidden;';
  link.download = `${filename}.${extension}`;

  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);

  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 5_000);
};
