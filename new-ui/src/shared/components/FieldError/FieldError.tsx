import { motion } from 'motion/react';
import './style.scss';
import { motionTransitionStandard } from '../../consts';
import { isPresent } from '../../utils/isPresent';

type Props = {
  error?: string | null;
};

export const FieldError = ({ error }: Props) => {
  return (
    <>
      {isPresent(error) && error.length > 0 && (
        <motion.p
          className="field-error"
          transition={motionTransitionStandard}
          initial={{
            y: -5,
            opacity: 0,
          }}
          animate={{
            y: 0,
            opacity: 1,
          }}
          exit={{
            y: -5,
            opacity: 0,
          }}
        >
          {error}
        </motion.p>
      )}
    </>
  );
};
