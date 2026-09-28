import './style.scss';
import { ArrowLeft, ArrowRight, Backspace, Delete, Enter } from '@fluentui/keyboard-keys';
import clsx from 'clsx';
import {
  type ClipboardEvent,
  type KeyboardEvent,
  useEffect,
  useRef,
  useState,
} from 'react';
import { isPresent } from '../../utils/isPresent';
import { FieldBox } from '../FieldBox/FieldBox';
import { FieldError } from '../FieldError/FieldError';

interface Props {
  length?: number;
  value: string | null;
  error?: string | null;
  onChange: (value: string) => void;
  /** Fired on Enter, on a full paste, and on the first time the code is fully typed. */
  onSubmit?: (value: string) => void;
  /** Blocks submitting while the previous submit is still being handled. */
  loading?: boolean;
}

const toDigits = (value: string | null, length: number): string[] => {
  const arr = Array.from({ length }, () => '');
  if (!value) return arr;
  for (let i = 0; i < Math.min(value.length, length); i++) {
    arr[i] = value[i] ?? '';
  }
  return arr;
};

export const CodeInput = ({
  onChange,
  value,
  error,
  onSubmit,
  loading = false,
  length = 6,
}: Props) => {
  const [digits, setDigits] = useState<string[]>(() => toDigits(value, length));
  const [focusedIndex, setFocusedIndex] = useState<number | null>(null);
  const inputRefs = useRef<(HTMLInputElement | null)[]>([]);
  const prevLengthRef = useRef(length);
  const hadErrorRef = useRef(false);
  const hasSubmittedRef = useRef(false);

  useEffect(() => {
    const frame = requestAnimationFrame(() => inputRefs.current[0]?.focus());
    return () => cancelAnimationFrame(frame);
  }, []);

  useEffect(() => {
    const lengthChanged = prevLengthRef.current !== length;
    prevLengthRef.current = length;

    if (lengthChanged) {
      hasSubmittedRef.current = false;
      setDigits(Array.from({ length }, () => ''));
      requestAnimationFrame(() => inputRefs.current[0]?.focus());
    } else {
      setDigits((current) => {
        if (current.join('') === (value ?? '')) return current;
        return toDigits(value, length);
      });
    }
  }, [value, length]);

  // Clears for retyping, callers must not treat the empty `onChange` as user input.
  useEffect(() => {
    const hasError = isPresent(error) && error.length > 0;
    const shouldClear = hasError && !hadErrorRef.current;
    hadErrorRef.current = hasError;

    if (shouldClear) {
      hasSubmittedRef.current = false;
      setDigits(Array.from({ length }, () => ''));
      onChange('');
      requestAnimationFrame(() => inputRefs.current[0]?.focus());
    }
  }, [error, length, onChange]);

  const focus = (index: number) => {
    const clamped = Math.max(0, Math.min(index, length - 1));
    inputRefs.current[clamped]?.focus();
  };

  const submit = (code: string) => {
    if (loading) return;
    hasSubmittedRef.current = true;
    onSubmit?.(code);
  };

  const updateDigit = (index: number, digit: string) => {
    setDigits((prev) => {
      const updated = [...prev];
      updated[index] = digit;
      onChange(updated.join(''));
      return updated;
    });
  };

  const handleKeyDown = (index: number, e: KeyboardEvent<HTMLInputElement>) => {
    if (e.ctrlKey || e.metaKey) return;

    if (e.key === Enter) {
      e.preventDefault();
      submit(digits.join(''));
    } else if (e.key === Backspace) {
      e.preventDefault();
      updateDigit(index, '');
      focus(index - 1);
    } else if (e.key === Delete) {
      e.preventDefault();
      updateDigit(index, '');
    } else if (e.key === ArrowLeft) {
      e.preventDefault();
      focus(index - 1);
    } else if (e.key === ArrowRight) {
      e.preventDefault();
      focus(index + 1);
    } else if (/^[0-9]$/.test(e.key)) {
      e.preventDefault();
      updateDigit(index, e.key);
      if (index < length - 1) {
        focus(index + 1);
      }
      const next = [...digits];
      next[index] = e.key;
      if (!hasSubmittedRef.current && next.every((d) => d !== '')) {
        submit(next.join(''));
      }
    } else if (e.key.length === 1) {
      e.preventDefault();
    }
  };

  const handlePaste = (e: ClipboardEvent<HTMLInputElement>) => {
    e.preventDefault();
    const cleaned = e.clipboardData.getData('text').trim().replace(/\D/g, '');
    if (cleaned.length === length) {
      const newDigits = cleaned.split('');
      setDigits(newDigits);
      onChange(cleaned);
      submit(cleaned);
      focus(length - 1);
    }
  };

  return (
    <div className="code-input">
      <div className="inputs-grid">
        {digits.map((digit, i) => (
          <FieldBox
            key={i}
            forceFocusState={focusedIndex === i}
            error={isPresent(error) && error.length > 0}
            size="default"
            onClick={() => {
              const input = inputRefs.current[i];
              if (input) {
                input.focus();
              }
            }}
          >
            <input
              ref={(el) => {
                inputRefs.current[i] = el;
              }}
              className={clsx({ empty: digit === '' })}
              type="text"
              inputMode="numeric"
              value={digit}
              onFocus={() => setFocusedIndex(i)}
              onBlur={() => setFocusedIndex(null)}
              onKeyDown={(e) => handleKeyDown(i, e)}
              onPaste={handlePaste}
              onChange={() => {}}
            />
          </FieldBox>
        ))}
      </div>
      <FieldError error={error} />
    </div>
  );
};
