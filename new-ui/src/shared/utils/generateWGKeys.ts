import { encode } from '@stablelib/base64';
import { generateKeyPair } from '@stablelib/x25519';

export const generateWGKeys = () => {
  const keys = generateKeyPair();
  return {
    publicKey: encode(keys.publicKey),
    privateKey: encode(keys.secretKey),
  };
};
