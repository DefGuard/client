const PasswordErrorCode = {
  Number: 'password_form_check_number',
  Special: 'password_form_check_special',
  Lowercase: 'password_form_check_lowercase',
  Uppercase: 'password_form_check_uppercase',
  Minimum: 'password_form_check_minimum',
} as const;

const errorCodes = Object.values(PasswordErrorCode);

export type PasswordErrorCodeValue =
  (typeof PasswordErrorCode)[keyof typeof PasswordErrorCode];

const errorIsCustomCode = (value: string): value is PasswordErrorCodeValue => {
  return (errorCodes as readonly string[]).includes(value);
};

const passwordErrorMessages: Record<PasswordErrorCodeValue, string> = {
  [PasswordErrorCode.Number]: 'At least one number required',
  [PasswordErrorCode.Special]: 'At least one special character',
  [PasswordErrorCode.Lowercase]: 'At least one lowercase character',
  [PasswordErrorCode.Uppercase]: 'At least one uppercase character',
  [PasswordErrorCode.Minimum]: 'Minimum length of 8',
};

export const passwordErrorMessage = (code: PasswordErrorCodeValue): string =>
  passwordErrorMessages[code];

export const mapPasswordFieldError = (
  errorValue: string,
  displayCustomError: boolean = false,
): string => {
  if (errorIsCustomCode(errorValue)) {
    if (displayCustomError) {
      return passwordErrorMessage(errorValue);
    }
    return 'Password does not meet requirements';
  }
  return errorValue;
};

const hasNumber = /[0-9]/;

const hasUppercase = /[A-Z]/;

const hasLowercase = /[a-z]/;

const hasSpecialChar = /[^a-zA-Z0-9]/;

export const refinePasswordField = (password: string): string[] => {
  const issues: string[] = [];
  if (password.length < 8) {
    issues.push(PasswordErrorCode.Minimum);
  }
  if (!hasNumber.test(password)) {
    issues.push(PasswordErrorCode.Number);
  }
  if (!hasUppercase.test(password)) {
    issues.push(PasswordErrorCode.Uppercase);
  }
  if (!hasLowercase.test(password)) {
    issues.push(PasswordErrorCode.Lowercase);
  }
  if (!hasSpecialChar.test(password)) {
    issues.push(PasswordErrorCode.Special);
  }
  return issues;
};
