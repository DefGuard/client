import { patternValidDomain, patternValidIp, patternValidIpV6 } from '../patterns';

// Returns flase when invalid
export const validateIpOrDomain = (val: string, allowMask = false): boolean => {
  return validateIp(val, allowMask) || patternValidDomain.test(val);
};

// Returns flase when invalid
export const validateIpList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
): boolean => {
  const trimed = val.replace(' ', '');
  const split = trimed.split(splitWith);
  return split.every((value) => validateIp(value, allowMasks));
};

// Returns flase when invalid
export const validateIpOrDomainList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
): boolean => {
  // split and trim values
  const split = val.split(splitWith).map((value) => value.trim());
  return split.every((value) => validateIpOrDomain(value, allowMasks));
};

// Returns flase when invalid
export const validateIp = (ip: string, allowMask = false): boolean => {
  if (allowMask) {
    if (ip.includes('/')) {
      const split = ip.split('/');
      if (split.length !== 2) return false;
      const ipValid = patternValidIp.test(split[0]);
      if (split[1] === '') return false;
      const mask = Number(split[1]);
      const maskValid = mask >= 0 && mask <= 32;
      return ipValid && maskValid;
    }
  }
  return patternValidIp.test(ip) || patternValidIpV6.test(ip);
};
