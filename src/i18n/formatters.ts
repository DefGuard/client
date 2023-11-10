import type { FormattersInitializer } from 'typesafe-i18n';

import type { Formatters,Locales } from './i18n-types';

// eslint-disable-next-line
export const initFormatters: FormattersInitializer<Locales, Formatters> = () => {

	const formatters: Formatters = {
		// add your formatter functions here
	};

	return formatters;
};
