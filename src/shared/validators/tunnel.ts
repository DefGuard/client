/* eslint-disable no-useless-escape */
export const patternValidDomain =
  /^(?:(?:(?:[a-zA-z\-]+)\:\/{1,3})?(?:[a-zA-Z0-9])(?:[a-zA-Z0-9\-\.]){1,61}(?:\.[a-zA-Z]{2,})+|\[(?:(?:(?:[a-fA-F0-9]){1,4})(?::(?:[a-fA-F0-9]){1,4}){7}|::1|::)\]|(?:(?:[0-9]{1,3})(?:\.[0-9]{1,3}){3}))(?:\:[0-9]{1,5})?$/;

export const patternValidIp =
  /^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;

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
  for (const value of split) {
    if (!validateIp(value, allowMasks)) {
      return false;
    }
  }
  return true;
};

// Returns flase when invalid
export const validateIpOrDomainList = (
  val: string,
  splitWith = ',',
  allowMasks = false,
): boolean => {
  const trimed = val.replace(' ', '');
  const split = trimed.split(splitWith);
  for (const value of split) {
    if (!(validateIp(value, allowMasks) && patternValidDomain.test(value))) {
      return false;
    }
  }
  return true;
};

// Returns flase when invalid
export const validateIp = (ip: string, allowMask = false): boolean => {
  if (allowMask) {
    if (ip.includes('/')) {
      const split = ip.split('/');
      if (split.length !== 2) return true;
      const ipValid = patternValidIp.test(split[0]);
      if (split[1] === '') return false;
      const mask = Number(split[1]);
      const maskValid = mask >= 0 && mask <= 32;
      return ipValid && maskValid;
    }
  }
  return patternValidIp.test(ip);
};
