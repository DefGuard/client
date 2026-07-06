import { $, expect } from '@wdio/globals';
import type { EnrollmentFixture } from './coreApi.js';
import { submitTotpCode } from './mfa.js';
import { switchToFullView } from './windows.js';

export const password = 'E2eTestPassword123!';

const clickNext = async () => {
  const next = $('.enroll-controls .right button');
  await next.waitForClickable();
  await next.click();
};

export const addInstance = async (fixture: EnrollmentFixture) => {
  await switchToFullView();
  const addCard = $('#add-page-view button');
  await addCard.waitForClickable();
  await addCard.click();
  await expect($('#add-instance-view')).toBeDisplayed();
  await $('[data-testid="field-url"]').setValue(fixture.enrollmentUrl);
  await $('[data-testid="field-token"]').setValue(fixture.enrollmentToken);
  const submit = $('#add-instance-view').$('button=Add Instance');
  await submit.waitForClickable();
  await submit.click();
  await expect($('#welcome-step')).toBeDisplayed();
};

export const setPassword = async () => {
  await clickNext();
  await expect($('#password-step')).toBeDisplayed();
  await $('[data-testid="field-password"]').setValue(password);
  await $('[data-testid="field-repeat"]').setValue(password);
  await clickNext();
};

export const configureTotp = async (): Promise<string> => {
  await expect($('#mfa-configuration-step')).toBeDisplayed();
  const secretField = $('#mfa-configuration-step .copy-field .track p');
  await secretField.waitForExist();
  const secret =
    ((await secretField.getProperty('textContent')) as string | null)?.trim() ?? '';
  await submitTotpCode(secret, '#mfa-configuration-step', clickNext, () =>
    $('#recovery-codes-step')
      .isDisplayed()
      .catch(() => false),
  );
  await $('#recovery-codes-step .checkbox').click();
  const complete = $('#recovery-codes-step').$('button=Complete');
  await complete.waitForClickable();
  await complete.click();
  return secret;
};

export const finishEnrollment = async () => {
  await expect($('#finish-step')).toBeDisplayed();
  await $('#finish-step').$('button=Got it').click();
  await expect($('#overview-page')).toBeDisplayed();
};
