import { save } from '@tauri-apps/plugin-dialog';
import { writeFile, writeTextFile } from '@tauri-apps/plugin-fs';

export const downloadText = async (
  content: string,
  filename: string,
  extension: 'txt' | 'pub' | 'conf' = 'txt',
): Promise<void> => {
  const path = await save({
    defaultPath: `${filename}.${extension}`,
    filters: [{ name: 'Text files', extensions: [extension] }],
  });
  if (path === null) return;
  await writeTextFile(path, content);
};

export const downloadFile = async (
  blob: Blob,
  filename: string,
  extension: string,
): Promise<void> => {
  const path = await save({
    defaultPath: `${filename}.${extension}`,
    filters: [{ name: 'Files', extensions: [extension] }],
  });
  if (path === null) return;
  const buffer = await blob.arrayBuffer();
  await writeFile(path, new Uint8Array(buffer));
};
