import type { Answer } from './app.contract.d.ts';

// The suite writes the id `exact new` gave the app.
export const appId = '__APP_ID__';
export const grants = 'device.notifications purpose.notifications\ndevice.push';
export const answer: Answer = (source) => { throw new Error(`no source ${String(source)}`); };
